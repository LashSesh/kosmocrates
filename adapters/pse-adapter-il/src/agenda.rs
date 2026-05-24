//! Epistemic agenda: prioritised action list derived from store health.
//!
//! The previous modules answer *what is the state of the crystal store*:
//! - `health`: uncertainty per crystal
//! - `lifecycle`: which crystals are stale or redundant
//! - `cluster`: knowledge topology (islands, bridges, singletons)
//! - `constitutional`: compliance violations
//!
//! The epistemic agenda answers *what should I do about it* — synthesising
//! all those signals into a ranked action list that an agent (or human
//! operator) can work through to move the store toward the health fixpoint.
//!
//! ## Action types
//!
//! | Action       | Source signal                             | Effect                         |
//! |--------------|-------------------------------------------|--------------------------------|
//! | `Refresh`    | Stale crystal (high decay + uncertainty)  | Re-ask the original question   |
//! | `Reinforce`  | At-risk crystal (high uncertainty)        | Improve quality / QTIC class   |
//! | `Consolidate`| Lifecycle consolidation candidate         | Remove redundant knowledge     |
//! | `Guard`      | Bridge crystal at risk                    | Protect critical connector     |
//! | `Explore`    | Causal root with low quality              | Fill knowledge gap             |
//!
//! ## Priority model
//!
//! Each item gets a priority score `p ∈ [0, 1]`:
//!
//! ```text
//! Blocking constitutional violation  → p = 1.00  (must fix)
//! Bridge crystal at risk             → p = 0.90 × uncertainty
//! Stale causal root                  → p = refresh_score × 0.85
//! Consolidation (metatron)           → p = 0.70
//! Consolidation (semantic)           → p = 0.60
//! At-risk crystal (non-root)         → p = uncertainty × 0.75
//! Stale non-root crystal             → p = refresh_score × 0.65
//! Cluster singleton                  → p = 0.30
//! ```
//!
//! Items are sorted by descending priority and capped at `max_items`.
//!
//! ## LLM integration
//!
//! [`EpistemicAgenda::to_context_block`] returns an `[AGENDA]...[/AGENDA]`
//! block suitable for injection into the LLM system message alongside the
//! PSE-CONTEXT block, giving the model a live action list to reason about.

use serde::{Deserialize, Serialize};

// ─── Action type ─────────────────────────────────────────────────────────────

/// The specific action recommended by an agenda item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgendaAction {
    /// Re-ask the question that originally generated this crystal — it has
    /// decayed or become uncertain and needs a fresh answer.
    Refresh {
        crystal_id: String,
        /// The original question stored in the crystal's index entry.
        original_question: String,
    },
    /// Improve the quality of a crystal that is uncertain but not yet stale.
    /// Typically done by running it through an IL refinement pass.
    Reinforce {
        crystal_id: String,
        uncertainty: f64,
    },
    /// Merge two structurally or semantically redundant crystals.
    /// Keep `retain`; deprecate `deprecate`.
    Consolidate {
        retain: String,
        deprecate: String,
        /// "metatron_isomorphic" or "semantic_overlap"
        reason: String,
    },
    /// A bridge crystal (critical cross-cluster connector) has elevated
    /// uncertainty — protect it before it decays further.
    Guard {
        crystal_id: String,
        bridges_clusters: usize,
        uncertainty: f64,
    },
    /// A causal root with below-threshold quality signals a knowledge gap —
    /// the foundation of a reasoning chain is weak or absent.
    Explore {
        /// The crystal_id of the low-quality root (for context).
        root_crystal_id: String,
        /// The stored question for that root crystal.
        topic_hint: String,
    },
}

impl AgendaAction {
    /// Short verb label for the action.
    pub fn verb(&self) -> &'static str {
        match self {
            Self::Refresh { .. } => "REFRESH",
            Self::Reinforce { .. } => "REINFORCE",
            Self::Consolidate { .. } => "CONSOLIDATE",
            Self::Guard { .. } => "GUARD",
            Self::Explore { .. } => "EXPLORE",
        }
    }

    /// Crystal ID affected by this action (first, when relevant).
    pub fn primary_id(&self) -> Option<&str> {
        match self {
            Self::Refresh { crystal_id, .. } => Some(crystal_id),
            Self::Reinforce { crystal_id, .. } => Some(crystal_id),
            Self::Consolidate { retain, .. } => Some(retain),
            Self::Guard { crystal_id, .. } => Some(crystal_id),
            Self::Explore {
                root_crystal_id, ..
            } => Some(root_crystal_id),
        }
    }
}

// ─── Agenda item ─────────────────────────────────────────────────────────────

/// A single actionable item in the epistemic agenda.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgendaItem {
    /// Priority score `p ∈ [0, 1]` (higher = more urgent).
    pub priority: f64,
    /// The recommended action.
    pub action: AgendaAction,
    /// Human-readable rationale explaining why this action is needed.
    pub rationale: String,
    /// Estimated reduction in store `mean_uncertainty` if this action
    /// is carried out successfully (0 = no impact estimate available).
    pub expected_uncertainty_delta: f64,
}

impl AgendaItem {
    /// One-line summary for LLM injection.
    pub fn to_line(&self) -> String {
        let id = self
            .action
            .primary_id()
            .map(|s| &s[..s.len().min(8)])
            .unwrap_or("—");
        format!(
            "[p={:.2}] {} {} — {}",
            self.priority,
            self.action.verb(),
            id,
            self.rationale,
        )
    }
}

// ─── Epistemic agenda ────────────────────────────────────────────────────────

/// Prioritised action list for moving the IL store toward the health fixpoint.
///
/// Built by [`ILStore::epistemic_agenda`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpistemicAgenda {
    /// Prioritised action items (descending priority, capped at `max_items`).
    pub items: Vec<AgendaItem>,
    /// One-line store state diagnosis (aggregated from health + lifecycle).
    pub diagnosis: String,
    /// Estimated number of items that, if resolved, would make the store
    /// healthy (`mean_uncertainty ≤ 0.30` and `fraction_q4_plus ≥ 0.80`).
    pub items_to_fixpoint: usize,
}

impl EpistemicAgenda {
    /// The top-k items by priority.
    pub fn top_k(&self, k: usize) -> &[AgendaItem] {
        let end = k.min(self.items.len());
        &self.items[..end]
    }

    /// True when the agenda is empty — the store has reached epistemic fixpoint.
    pub fn is_fixpoint(&self) -> bool {
        self.items.is_empty()
    }

    /// Build an `[AGENDA]...[/AGENDA]` block for LLM system message injection.
    ///
    /// Includes the diagnosis and the top items, formatted for readability.
    pub fn to_context_block(&self, top_k: usize) -> String {
        if self.is_fixpoint() {
            return "[AGENDA]\nEpistemic fixpoint reached — no actions required.\n[/AGENDA]"
                .to_string();
        }
        let mut out = String::from("[AGENDA]\n");
        out.push_str(&format!("diagnosis: {}\n", self.diagnosis));
        out.push_str(&format!(
            "items_to_fixpoint: {} | total: {}\n",
            self.items_to_fixpoint,
            self.items.len()
        ));
        for item in self.top_k(top_k) {
            out.push_str(&item.to_line());
            out.push('\n');
        }
        out.push_str("[/AGENDA]");
        out
    }
}

// ─── AgendaConfig ─────────────────────────────────────────────────────────────

/// Configuration for [`ILStore::epistemic_agenda`].
#[derive(Debug, Clone)]
pub struct AgendaConfig {
    /// Decay model for lifecycle staleness analysis.
    pub decay_model: crate::lifecycle::DecayModel,
    /// Similarity threshold for consolidation candidate detection.
    pub sim_threshold: f64,
    /// Uncertainty threshold above which a crystal is "at risk".
    /// Default: 0.5.
    pub at_risk_threshold: f64,
    /// Maximum number of items to include in the agenda.
    pub max_items: usize,
}

impl Default for AgendaConfig {
    fn default() -> Self {
        Self {
            decay_model: crate::lifecycle::DecayModel::Exponential { half_life: 100.0 },
            sim_threshold: 0.80,
            at_risk_threshold: 0.50,
            max_items: 20,
        }
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(priority: f64, action: AgendaAction) -> AgendaItem {
        AgendaItem {
            priority,
            action,
            rationale: "test rationale".to_string(),
            expected_uncertainty_delta: 0.05,
        }
    }

    #[test]
    fn agenda_action_verb_labels() {
        assert_eq!(
            AgendaAction::Refresh {
                crystal_id: "a".into(),
                original_question: "q".into()
            }
            .verb(),
            "REFRESH"
        );
        assert_eq!(
            AgendaAction::Reinforce {
                crystal_id: "a".into(),
                uncertainty: 0.7
            }
            .verb(),
            "REINFORCE"
        );
        assert_eq!(
            AgendaAction::Consolidate {
                retain: "a".into(),
                deprecate: "b".into(),
                reason: "metatron_isomorphic".into()
            }
            .verb(),
            "CONSOLIDATE"
        );
        assert_eq!(
            AgendaAction::Guard {
                crystal_id: "a".into(),
                bridges_clusters: 2,
                uncertainty: 0.6
            }
            .verb(),
            "GUARD"
        );
        assert_eq!(
            AgendaAction::Explore {
                root_crystal_id: "a".into(),
                topic_hint: "memory".into()
            }
            .verb(),
            "EXPLORE"
        );
    }

    #[test]
    fn agenda_item_to_line_format() {
        let item = make_item(
            0.87,
            AgendaAction::Refresh {
                crystal_id: "abcdef1234567890".into(),
                original_question: "What is memory?".into(),
            },
        );
        let line = item.to_line();
        assert!(line.contains("[p=0.87]"));
        assert!(line.contains("REFRESH"));
        assert!(line.contains("abcdef12")); // first 8 chars of crystal_id
    }

    #[test]
    fn empty_agenda_is_fixpoint() {
        let agenda = EpistemicAgenda {
            items: vec![],
            diagnosis: "store is healthy".to_string(),
            items_to_fixpoint: 0,
        };
        assert!(agenda.is_fixpoint());
        assert!(agenda.to_context_block(5).contains("fixpoint"));
    }

    #[test]
    fn non_empty_agenda_not_fixpoint() {
        let agenda = EpistemicAgenda {
            items: vec![make_item(
                0.8,
                AgendaAction::Reinforce {
                    crystal_id: "abc".into(),
                    uncertainty: 0.8,
                },
            )],
            diagnosis: "1 crystal at risk".to_string(),
            items_to_fixpoint: 1,
        };
        assert!(!agenda.is_fixpoint());
    }

    #[test]
    fn top_k_caps_at_available() {
        let items: Vec<AgendaItem> = (0..3)
            .map(|i| {
                make_item(
                    i as f64 / 10.0,
                    AgendaAction::Reinforce {
                        crystal_id: format!("crystal{i}"),
                        uncertainty: 0.5,
                    },
                )
            })
            .collect();
        let agenda = EpistemicAgenda {
            items,
            diagnosis: "test".to_string(),
            items_to_fixpoint: 3,
        };
        assert_eq!(agenda.top_k(10).len(), 3); // capped at available
        assert_eq!(agenda.top_k(2).len(), 2);
    }

    #[test]
    fn context_block_structure() {
        let agenda = EpistemicAgenda {
            items: vec![make_item(
                0.9,
                AgendaAction::Guard {
                    crystal_id: "bridge001".into(),
                    bridges_clusters: 2,
                    uncertainty: 0.6,
                },
            )],
            diagnosis: "1 bridge crystal at risk".to_string(),
            items_to_fixpoint: 1,
        };
        let block = agenda.to_context_block(5);
        assert!(block.starts_with("[AGENDA]"));
        assert!(block.ends_with("[/AGENDA]"));
        assert!(block.contains("diagnosis:"));
        assert!(block.contains("GUARD"));
    }

    #[test]
    fn default_config_sensible() {
        let cfg = AgendaConfig::default();
        assert_eq!(cfg.max_items, 20);
        assert!((cfg.at_risk_threshold - 0.5).abs() < 1e-9);
        assert!((cfg.sim_threshold - 0.80).abs() < 1e-9);
    }
}

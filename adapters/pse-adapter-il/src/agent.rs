//! Multi-agent crystal attribution and cross-agent causal analysis.
//!
//! Each crystal committed via [`ILStore::commit_as`] carries an `agent_id`
//! tag in the index.  [`ILStore::build_agent_causal_graph`] then lifts the
//! raw causal graph into an annotated [`AgentCausalGraph`] that reveals:
//!
//! - Which agent produced each crystal.
//! - Which causal links cross agent boundaries.
//! - Which crystals emerged from multi-agent collaboration (i.e., have
//!   ancestors from at least two distinct agents).
//!
//! ## Why `universe_id`?
//! `SemanticCrystal::universe_id` is a string field reserved for namespacing.
//! `commit_as` uses it to carry the agent identifier into the block file, so
//! the attribution survives ledger replay even without the IL index.
//!
//! ## Example
//! ```ignore
//! // crystal obtained from pse_core::macro_step or similar
//! store.commit_as(&crystal, &[], 1, "Agent-A query", "agent-a").unwrap();
//!
//! let g = store.build_agent_causal_graph();
//! println!("{}", g.summary());
//! println!("Collaborative crystals: {:?}", g.collaborative_crystals());
//! ```

use crate::causal::{CausalGraph, CausalLink};
use std::collections::{HashMap, HashSet};

/// A causal link annotated with agent attribution.
#[derive(Debug, Clone)]
pub struct AgentLink {
    /// The underlying causal link (crystal IDs + cause + strength).
    pub inner: CausalLink,
    /// Agent that produced the source crystal (empty = untagged).
    pub from_agent: String,
    /// Agent that produced the target crystal (empty = untagged).
    pub to_agent: String,
}

impl AgentLink {
    /// True when source and target crystals were produced by different, known agents.
    pub fn is_cross_agent(&self) -> bool {
        !self.from_agent.is_empty()
            && !self.to_agent.is_empty()
            && self.from_agent != self.to_agent
    }
}

/// Causal graph annotated with per-crystal agent attribution.
///
/// Built by [`ILStore::build_agent_causal_graph`].
#[derive(Debug, Clone, Default)]
pub struct AgentCausalGraph {
    /// All causal links, each annotated with agent IDs.
    pub links: Vec<AgentLink>,
    /// All known agent IDs (crystals with a non-empty agent tag).
    pub agents: HashSet<String>,
    /// Crystal ID prefix (16 chars) → agent ID.
    pub attribution: HashMap<String, String>,
}

impl AgentCausalGraph {
    /// Only the cross-agent links (source and target from different agents).
    pub fn cross_agent_links(&self) -> Vec<&AgentLink> {
        self.links.iter().filter(|l| l.is_cross_agent()).collect()
    }

    /// Crystals that have ancestors contributed by at least two distinct agents.
    ///
    /// These are "collaborative" crystals — their causal chain spans agent
    /// boundaries, indicating convergent knowledge integration.
    pub fn collaborative_crystals(&self) -> Vec<String> {
        let inner_graph = CausalGraph {
            links: self.links.iter().map(|l| l.inner.clone()).collect(),
        };
        let all_targets: HashSet<&str> =
            self.links.iter().map(|l| l.inner.to.as_str()).collect();

        all_targets
            .into_iter()
            .filter(|&crystal_id| {
                let ancestors = inner_graph.ancestors(crystal_id);
                let ancestor_agents: HashSet<&str> = ancestors
                    .iter()
                    .filter_map(|anc| self.attribution.get(anc.as_str()).map(|s| s.as_str()))
                    .filter(|a| !a.is_empty())
                    .collect();
                ancestor_agents.len() >= 2
            })
            .map(|s| s.to_string())
            .collect()
    }

    /// Crystals directly attributed to a specific agent.
    pub fn crystals_by(&self, agent_id: &str) -> Vec<String> {
        self.attribution
            .iter()
            .filter(|(_, a)| a.as_str() == agent_id)
            .map(|(c, _)| c.clone())
            .collect()
    }

    /// The agent with the most outgoing causal links (most causally active).
    pub fn most_active_agent(&self) -> Option<&str> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for l in &self.links {
            if !l.from_agent.is_empty() {
                *counts.entry(l.from_agent.as_str()).or_insert(0) += 1;
            }
        }
        counts
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|(a, _)| a)
    }

    /// Human-readable summary.
    pub fn summary(&self) -> String {
        let cross = self.cross_agent_links().len();
        let collab = self.collaborative_crystals().len();
        if self.links.is_empty() {
            return "AgentCausalGraph: empty".to_string();
        }
        format!(
            "AgentCausalGraph: {} agent(s), {} links total, {} cross-agent, {} collaborative crystal(s)",
            self.agents.len(),
            self.links.len(),
            cross,
            collab
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::causal::{CausalCause, CausalLink};

    fn make_link(from: &str, to: &str) -> CausalLink {
        CausalLink {
            from: from.to_string(),
            to: to.to_string(),
            cause: CausalCause::SequentialCommit,
            strength: 0.7,
        }
    }

    fn agent_link(from: &str, to: &str, from_agent: &str, to_agent: &str) -> AgentLink {
        AgentLink {
            inner: make_link(from, to),
            from_agent: from_agent.to_string(),
            to_agent: to_agent.to_string(),
        }
    }

    fn make_graph(links: Vec<AgentLink>, attr: &[(&str, &str)]) -> AgentCausalGraph {
        let attribution: HashMap<String, String> =
            attr.iter().map(|(c, a)| (c.to_string(), a.to_string())).collect();
        let agents: HashSet<String> =
            attribution.values().filter(|a| !a.is_empty()).cloned().collect();
        AgentCausalGraph { links, agents, attribution }
    }

    #[test]
    fn cross_agent_link_detected() {
        let link = agent_link("aaa", "bbb", "alice", "bob");
        assert!(link.is_cross_agent(), "different agents must be cross-agent");
    }

    #[test]
    fn same_agent_link_not_cross_agent() {
        let link = agent_link("aaa", "bbb", "alice", "alice");
        assert!(!link.is_cross_agent(), "same agent must not be cross-agent");
    }

    #[test]
    fn empty_agent_not_cross_agent() {
        let link = agent_link("aaa", "bbb", "", "alice");
        assert!(!link.is_cross_agent(), "untagged source must not be cross-agent");
    }

    #[test]
    fn cross_agent_links_filters_correctly() {
        let links = vec![
            agent_link("aaa", "bbb", "alice", "bob"),   // cross
            agent_link("bbb", "ccc", "bob", "bob"),      // same
            agent_link("ccc", "ddd", "carol", "alice"), // cross
        ];
        let g = make_graph(
            links,
            &[("aaa", "alice"), ("bbb", "bob"), ("ccc", "bob"), ("ddd", "alice")],
        );
        assert_eq!(g.cross_agent_links().len(), 2);
    }

    #[test]
    fn crystals_by_agent() {
        let links = vec![agent_link("aaa", "bbb", "alice", "bob")];
        let g = make_graph(links, &[("aaa", "alice"), ("bbb", "bob")]);
        let alices = g.crystals_by("alice");
        assert_eq!(alices, vec!["aaa"]);
        let bobs = g.crystals_by("bob");
        assert_eq!(bobs, vec!["bbb"]);
    }

    #[test]
    fn collaborative_crystal_has_multi_agent_ancestors() {
        // Chain: alice:aaa → alice:bbb → ccc, bob:ddd → ccc
        // ccc has ancestors from both alice and bob → collaborative
        let links = vec![
            agent_link("aaa", "bbb", "alice", "alice"),
            agent_link("bbb", "ccc", "alice", "carol"),
            agent_link("ddd", "ccc", "bob", "carol"),
        ];
        let g = make_graph(
            links,
            &[
                ("aaa", "alice"),
                ("bbb", "alice"),
                ("ccc", "carol"),
                ("ddd", "bob"),
            ],
        );
        let collab = g.collaborative_crystals();
        assert!(collab.contains(&"ccc".to_string()), "ccc must be collaborative");
    }

    #[test]
    fn non_collaborative_crystal_excluded() {
        // Linear chain: alice → alice → alice — no cross-agent ancestry
        let links = vec![
            agent_link("aaa", "bbb", "alice", "alice"),
            agent_link("bbb", "ccc", "alice", "alice"),
        ];
        let g = make_graph(
            links,
            &[("aaa", "alice"), ("bbb", "alice"), ("ccc", "alice")],
        );
        assert!(g.collaborative_crystals().is_empty());
    }

    #[test]
    fn most_active_agent_with_most_outgoing_links() {
        let links = vec![
            agent_link("a1", "b1", "alice", "bob"),
            agent_link("a2", "b2", "alice", "carol"),
            agent_link("d1", "b3", "dave", "carol"),
        ];
        let g = make_graph(
            links,
            &[("a1", "alice"), ("a2", "alice"), ("d1", "dave")],
        );
        assert_eq!(g.most_active_agent(), Some("alice"));
    }

    #[test]
    fn summary_includes_agent_count() {
        let links = vec![agent_link("aaa", "bbb", "alice", "bob")];
        let g = make_graph(links, &[("aaa", "alice"), ("bbb", "bob")]);
        let s = g.summary();
        assert!(s.contains("2 agent"), "summary must report 2 agents");
        assert!(s.contains("cross-agent"));
    }

    #[test]
    fn empty_graph_summary() {
        let g = AgentCausalGraph::default();
        assert!(g.summary().contains("empty"));
    }
}

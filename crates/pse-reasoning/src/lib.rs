//! Epistemic Thunderbolt Vector — D = ψ · ρ · ω guided reasoning.
//!
//! The Pfauenthron++ retrieval formula `D = ψ · ρ · ω` is not just a
//! retrieval metric — it is a *generalized epistemic energy function*.
//!
//! Applied to reasoning:
//!
//! - **Retrieval**: select the highest-D crystal from the store once.
//! - **Reasoning**: follow the gradient of D *across multiple hops* through
//!   the knowledge graph.  Each step uses the previous crystal's `vector8`
//!   as the next query, tracing a path of maximum epistemic coherence.
//!
//! The metaphor: lightning follows the path of least electrical resistance.
//! The Epistemic Thunderbolt follows the path of highest epistemic energy
//! through the IL knowledge graph — it is *attractor-constrained reasoning*.
//!
//! ## Algorithm
//!
//! ```text
//! query_text
//!   │
//!   └─ text_to_vector8(query) ──► current_vec
//!
//! loop step 0..max_steps:
//!   hits = score_tripolar(current_vec)        // D = ψ · ρ · ω for all crystals
//!   best = first hit not already in chain      // loop prevention
//!   if best.D < min_d_threshold  → terminate(MinThreshold)
//!   chain.push(best)
//!   current_vec = crystal_vector8(best.id)    // follow the crystal's embedding
//!
//! terminate(MaxSteps)
//! ```
//!
//! The chain terminates when:
//! - `max_steps` is reached (configurable, default 6)
//! - The highest available D drops below `min_d_threshold` (default 0.01)
//! - No unvisited crystals remain (loop exhaustion)

use pse_adapter_il::{adapter::text_to_vector8, store::ILStore};
use serde::{Deserialize, Serialize};

// ── Configuration ─────────────────────────────────────────────────────────────

/// Configuration for a single Thunderbolt reasoning run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThunderboltConfig {
    /// Maximum number of hops through the knowledge graph.
    pub max_steps: usize,
    /// Stop when the best available D drops below this value.
    pub min_d_threshold: f64,
    /// How many candidates to score per step (deeper search = slower).
    pub top_k_per_step: usize,
}

impl Default for ThunderboltConfig {
    fn default() -> Self {
        Self {
            max_steps: 6,
            min_d_threshold: 0.01,
            top_k_per_step: 32,
        }
    }
}

// ── Step ──────────────────────────────────────────────────────────────────────

/// A single hop in the reasoning chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step_index: usize,
    pub crystal_id_hex: String,
    /// D = ψ · ρ · ω for this hop.
    pub d_score: f64,
    /// D summed over all steps so far (trajectory energy).
    pub cumulative_d: f64,
    /// QTIC conformance class of this crystal (0–5).
    pub qtic_class: u8,
    /// PSE stability score ρ (also the ρ factor in D).
    pub stability_score: f64,
    /// True when this crystal is below the exploratory ψ boundary.
    /// Exploratory steps are valid but represent ungrounded hypotheses.
    pub is_exploratory: bool,
}

// ── Termination ───────────────────────────────────────────────────────────────

/// Why the chain stopped.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TerminationReason {
    /// Chain reached `max_steps` — budget exhausted.
    MaxSteps,
    /// Best available D dropped below `min_d_threshold` — signal faded.
    MinThreshold,
    /// All reachable crystals already visited — graph exhausted.
    NoNewMatches,
    /// IL store is empty — no knowledge to traverse.
    EmptyStore,
}

// ── Chain ─────────────────────────────────────────────────────────────────────

/// A complete Thunderbolt reasoning chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningChain {
    pub query: String,
    pub steps: Vec<ReasoningStep>,
    /// Sum of D scores across all steps.
    pub total_d: f64,
    /// Mean D per step.
    pub mean_d: f64,
    pub terminated_by: TerminationReason,
    /// True when any step in the chain is exploratory (ψ < 0 proxy).
    pub has_exploratory_steps: bool,
}

impl ReasoningChain {
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The peak D value across all steps (energy of the strongest attractor hit).
    pub fn peak_d(&self) -> f64 {
        self.steps.iter().map(|s| s.d_score).fold(0.0_f64, f64::max)
    }

    /// Mean QTIC class across all steps — epistemic quality of the chain.
    pub fn mean_qtic(&self) -> f64 {
        if self.steps.is_empty() {
            return 0.0;
        }
        self.steps.iter().map(|s| s.qtic_class as f64).sum::<f64>() / self.steps.len() as f64
    }
}

// ── Core reasoning function ───────────────────────────────────────────────────

/// Run the Epistemic Thunderbolt algorithm against `store`.
///
/// Returns a `ReasoningChain` tracing the highest-D path from `query`
/// through the IL knowledge graph.
///
/// Each step:
/// 1. Scores all crystals with `D = ψ · ρ · ω` (via `score_tripolar`)
/// 2. Selects the highest-D unvisited crystal
/// 3. Checks whether D is above `min_d_threshold`
/// 4. Advances the query vector to the crystal's stored `vector8`
///
/// The chain is deterministic for a given store state.
pub fn guide(query: &str, store: &ILStore, config: &ThunderboltConfig) -> ReasoningChain {
    if store.is_empty() {
        return ReasoningChain {
            query: query.to_string(),
            steps: vec![],
            total_d: 0.0,
            mean_d: 0.0,
            terminated_by: TerminationReason::EmptyStore,
            has_exploratory_steps: false,
        };
    }

    let mut current_vec = text_to_vector8(query);
    let mut visited: std::collections::HashSet<String> = Default::default();
    let mut steps: Vec<ReasoningStep> = Vec::with_capacity(config.max_steps);
    let mut terminated_by = TerminationReason::MaxSteps;

    for step_idx in 0..config.max_steps {
        // Score all crystals with D = ψ · ρ · ω.
        let mut hits = store.score_tripolar(&current_vec);
        hits.truncate(config.top_k_per_step);

        // Select the first unvisited hit.
        let candidate = hits.into_iter().find(|h| !visited.contains(&h.crystal_id_hex));

        let Some(hit) = candidate else {
            terminated_by = TerminationReason::NoNewMatches;
            break;
        };

        if hit.score < config.min_d_threshold {
            terminated_by = TerminationReason::MinThreshold;
            break;
        }

        visited.insert(hit.crystal_id_hex.clone());

        let (qtic_class, stability_score) = store
            .crystal_meta(&hit.crystal_id_hex)
            .unwrap_or((0, 0.5));

        // Exploratory proxy: low QTIC (Q0 or Q1) signals ungrounded crystal.
        let is_exploratory = qtic_class <= 1;

        let cumulative = steps.last().map(|s| s.cumulative_d).unwrap_or(0.0) + hit.score;

        steps.push(ReasoningStep {
            step_index: step_idx,
            crystal_id_hex: hit.crystal_id_hex.clone(),
            d_score: hit.score,
            cumulative_d: cumulative,
            qtic_class,
            stability_score,
            is_exploratory,
        });

        // Advance: follow the crystal's 8D embedding as the next query vector.
        match store.crystal_vector8(&hit.crystal_id_hex) {
            Some(v) => current_vec = v,
            None => {
                // Crystal disappeared between score and lookup (race condition in tests).
                // Re-run from query vector rather than silently looping.
                current_vec = text_to_vector8(query);
            }
        }
    }

    let total_d: f64 = steps.iter().map(|s| s.d_score).sum();
    let mean_d = if steps.is_empty() {
        0.0
    } else {
        total_d / steps.len() as f64
    };
    let has_exploratory_steps = steps.iter().any(|s| s.is_exploratory);

    ReasoningChain {
        query: query.to_string(),
        steps,
        total_d,
        mean_d,
        terminated_by,
        has_exploratory_steps,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use pse_adapter_il::store::ILStore;
    use pse_types::{
        CommitProof, ConstraintProgram, EvidenceChain, SemanticCrystal, TopologySignature,
    };
    use std::collections::BTreeMap;

    fn make_crystal(id: &str, stability: f64, kuramoto: f64) -> SemanticCrystal {
        use pse_types::Hash256;
        let mut hash: Hash256 = [0u8; 32];
        for (i, b) in id.bytes().enumerate().take(32) { hash[i] = b; }
        SemanticCrystal {
            crystal_id: hash,
            region: vec![],
            constraint_program: ConstraintProgram::default(),
            stability_score: stability,
            topology_signature: TopologySignature {
                kuramoto_coherence: kuramoto,
                betti_0: 1,
                euler_char: 1,
                ..TopologySignature::default()
            },
            betti_numbers: vec![1],
            evidence_chain: EvidenceChain::default(),
            commit_proof: CommitProof::default(),
            operator_versions: BTreeMap::new(),
            created_at: 0,
            free_energy: (1.0 - stability).powi(2),
            carrier_instance_idx: 0,
            scale_tag: "test".to_string(),
            universe_id: "test-v1".to_string(),
            sub_crystal_ids: vec![],
            parent_crystal_ids: vec![],
            genesis_metadata: None,
            metatron_signature: None,
        }
    }

    fn open_store(dir: &std::path::Path) -> ILStore {
        ILStore::open(dir, "test-seed").expect("open store")
    }

    fn commit(store: &mut ILStore, crystal: SemanticCrystal, question: &str) {
        let chunks = vec![question.to_string()];
        let _ = store.commit_with_feedback(&crystal, &chunks, 1, question);
    }

    // ── Empty store ────────────────────────────────────────────────────────

    #[test]
    fn empty_store_returns_empty_chain() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let chain = guide("test query", &store, &ThunderboltConfig::default());
        assert!(chain.is_empty());
        assert_eq!(chain.terminated_by, TerminationReason::EmptyStore);
    }

    // ── Single crystal ─────────────────────────────────────────────────────

    #[test]
    fn single_crystal_yields_one_step() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        commit(&mut store, make_crystal("c1", 0.85, 0.75), "test query stability");

        let config = ThunderboltConfig { max_steps: 3, ..Default::default() };
        let chain = guide("stability", &store, &config);

        // With one crystal the chain exhausts after one step.
        assert!(!chain.is_empty());
        assert_eq!(chain.steps[0].d_score, chain.total_d);
    }

    // ── Multi-crystal traversal ────────────────────────────────────────────

    #[test]
    fn multi_crystal_traversal_no_repeats() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        commit(&mut store, make_crystal("c1", 0.9, 0.8), "epistemic coherence");
        commit(&mut store, make_crystal("c2", 0.7, 0.6), "stability attractor");
        commit(&mut store, make_crystal("c3", 0.5, 0.4), "free energy gradient");

        let config = ThunderboltConfig { max_steps: 10, min_d_threshold: 1e-6, ..Default::default() };
        let chain = guide("coherence", &store, &config);

        // No crystal should appear twice.
        let ids: Vec<&str> = chain.steps.iter().map(|s| s.crystal_id_hex.as_str()).collect();
        let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len(), "reasoning chain must not revisit crystals");
    }

    // ── Termination ────────────────────────────────────────────────────────

    #[test]
    fn terminates_at_max_steps() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        for i in 0..10 {
            commit(&mut store, make_crystal(&format!("c{i}"), 0.8, 0.7), &format!("concept {i}"));
        }
        let config = ThunderboltConfig { max_steps: 3, min_d_threshold: 0.0, ..Default::default() };
        let chain = guide("concept", &store, &config);
        assert!(chain.steps.len() <= 3);
        if chain.steps.len() == 3 {
            assert_eq!(chain.terminated_by, TerminationReason::MaxSteps);
        }
    }

    #[test]
    fn terminates_below_min_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        // Low stability + low kuramoto → low D.
        commit(&mut store, make_crystal("c1", 0.1, 0.1), "very weak signal here");

        let config = ThunderboltConfig {
            max_steps: 10,
            min_d_threshold: 0.99,  // impossibly high
            ..Default::default()
        };
        let chain = guide("unrelated topic", &store, &config);
        // Either empty (first hit below threshold) or terminated_by MinThreshold.
        assert!(
            chain.is_empty() || chain.terminated_by == TerminationReason::MinThreshold,
            "high threshold should trigger MinThreshold termination"
        );
    }

    // ── Chain metrics ──────────────────────────────────────────────────────

    #[test]
    fn total_d_equals_sum_of_steps() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        commit(&mut store, make_crystal("c1", 0.8, 0.7), "alpha");
        commit(&mut store, make_crystal("c2", 0.6, 0.5), "beta");

        let chain = guide("alpha beta", &store, &ThunderboltConfig::default());
        let sum: f64 = chain.steps.iter().map(|s| s.d_score).sum();
        assert!((chain.total_d - sum).abs() < 1e-10);
    }

    #[test]
    fn cumulative_d_is_non_decreasing() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        for i in 0..5 {
            commit(&mut store, make_crystal(&format!("c{i}"), 0.7, 0.6), "knowledge");
        }
        let chain = guide("knowledge", &store, &ThunderboltConfig::default());
        let cumulative: Vec<f64> = chain.steps.iter().map(|s| s.cumulative_d).collect();
        for w in cumulative.windows(2) {
            assert!(w[1] >= w[0], "cumulative_d must be non-decreasing");
        }
    }

    #[test]
    fn peak_d_is_max_step_score() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        commit(&mut store, make_crystal("c1", 0.9, 0.9), "strong signal");
        commit(&mut store, make_crystal("c2", 0.1, 0.1), "weak signal");

        let chain = guide("signal", &store, &ThunderboltConfig::default());
        if !chain.is_empty() {
            let max = chain.steps.iter().map(|s| s.d_score).fold(0.0_f64, f64::max);
            assert!((chain.peak_d() - max).abs() < 1e-10);
        }
    }

    // ── crystal_vector8 accessor ───────────────────────────────────────────

    #[test]
    fn vector8_accessor_returns_none_for_unknown_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        assert!(store.crystal_vector8("nonexistent").is_none());
    }

    #[test]
    fn vector8_accessor_returns_vector_after_commit() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        commit(&mut store, make_crystal("cx", 0.8, 0.7), "test");

        // Find the crystal_id_hex by running score_tripolar.
        let hits = store.score_tripolar(&text_to_vector8("test"));
        assert!(!hits.is_empty());
        let hex = &hits[0].crystal_id_hex;
        let vec = store.crystal_vector8(hex);
        assert!(vec.is_some());
        let v = vec.unwrap();
        assert_eq!(v.len(), 8);
        // L2-normalized: norm should be ≈ 1.
        let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "vector8 must be L2-normalized");
    }
}

//! Blueprint search — deterministic generation of [`TraversalBlueprint`]s
//! and a non-dominated frontier tracker.
//!
//! Implements spec PSE-TRAVERSE-SIGNATURE-01 §11.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::canonical::hex_address;
use crate::operator::{OperatorBuildConfig, OperatorKind};
use crate::signature::SignatureConfig;
use crate::signature_gate::SignatureGateConfig;

// ──────────────────────────────────────────────────────────────────────────────
// Policy enums
// ──────────────────────────────────────────────────────────────────────────────

/// Controls how nodes / edges are sampled or represented.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TopologyPolicy {
    Default,
    Sparse,
    Dense,
}

/// Controls how collapse decisions are prioritised.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CollapsePolicy {
    Default,
    AggressiveHard,
    SoftFirst,
}

// ──────────────────────────────────────────────────────────────────────────────
// TraversalBlueprint
// ──────────────────────────────────────────────────────────────────────────────

/// A fully-specified configuration bundle for one traversal attempt.
///
/// `blueprint_id` is content-addressed (derived with the field set to "").
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TraversalBlueprint {
    /// Content-addressed identifier — derived with this field set to "".
    pub blueprint_id: String,
    /// Hash of the `ProblemSpec` this blueprint targets.
    pub problem_spec_hash: String,
    pub topology_policy: TopologyPolicy,
    pub collapse_policy: CollapsePolicy,
    pub operator_policy: OperatorBuildConfig,
    pub signature_policy: SignatureConfig,
    pub gate_policy: SignatureGateConfig,
    pub seed: u64,
    pub parent_blueprint_id: Option<String>,
}

// ──────────────────────────────────────────────────────────────────────────────
// BlueprintSearchPolicy
// ──────────────────────────────────────────────────────────────────────────────

/// Strategy used by [`BlueprintSearch`] to generate candidate blueprints.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlueprintSearchPolicy {
    /// Exhaustively try all combinations on a fixed grid.
    DeterministicGrid,
    /// Select the top-K scoring blueprints from the frontier.
    TopK,
    /// Upper confidence bound exploration/exploitation.
    Ucb,
    /// ε-greedy exploration/exploitation.
    EpsilonGreedy,
    /// Weight candidates by novelty relative to the current frontier.
    NoveltyWeighted,
}

// ──────────────────────────────────────────────────────────────────────────────
// NonDominatedFrontier
// ──────────────────────────────────────────────────────────────────────────────

/// Tracks a Pareto-optimal front of blueprints under three objectives:
/// - `gap_score`: higher is better
/// - `fragmentation`: lower is better
/// - `degeneracy`: lower is better
///
/// Internally, fragmentation and degeneracy are negated so that "higher is
/// better" applies uniformly to all three coordinates.
pub struct NonDominatedFrontier {
    /// `(blueprint_id, scores)` where scores are stored as provided.
    entries: Vec<(String, BTreeMap<String, f64>)>,
}

impl NonDominatedFrontier {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Add a blueprint to the frontier if it is not dominated.
    ///
    /// Blueprint **a** dominates blueprint **b** iff a ≥ b in all three
    /// metrics AND a > b in at least one.  Metrics used: `gap_score`,
    /// `fragmentation_score`, `degeneracy_score`.
    pub fn add(&mut self, blueprint_id: String, scores: BTreeMap<String, f64>) {
        // Convert scores to comparable triple (all higher = better).
        let coords = Self::to_coords(&scores);

        // Check if the new entry is dominated by an existing one.
        for (_, existing_scores) in &self.entries {
            let ec = Self::to_coords(existing_scores);
            if dominates(ec, coords) {
                // New entry is dominated — discard.
                return;
            }
        }

        // Remove existing entries that are dominated by the new entry.
        self.entries.retain(|(_, existing_scores)| {
            let ec = Self::to_coords(existing_scores);
            !dominates(coords, ec)
        });

        self.entries.push((blueprint_id, scores));
    }

    /// Return sorted blueprint IDs on the frontier.
    pub fn frontier_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.entries.iter().map(|(id, _)| id.clone()).collect();
        ids.sort();
        ids
    }

    /// Convert a score map to a [gap, -fragmentation, -degeneracy] triple.
    fn to_coords(scores: &BTreeMap<String, f64>) -> [f64; 3] {
        let gap = *scores.get("gap_score").unwrap_or(&0.0);
        let frag = *scores.get("fragmentation_score").unwrap_or(&0.0);
        let deg = *scores.get("degeneracy_score").unwrap_or(&0.0);
        [gap, -frag, -deg]
    }
}

/// Returns true if `a` dominates `b`: a[i] >= b[i] for all i, and a[i] > b[i]
/// for at least one i.
fn dominates(a: [f64; 3], b: [f64; 3]) -> bool {
    let all_ge = a.iter().zip(b.iter()).all(|(&ai, &bi)| ai >= bi);
    let any_gt = a.iter().zip(b.iter()).any(|(&ai, &bi)| ai > bi);
    all_ge && any_gt
}

impl Default for NonDominatedFrontier {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// BlueprintSearch
// ──────────────────────────────────────────────────────────────────────────────

/// Generates candidate [`TraversalBlueprint`]s according to a search policy.
pub struct BlueprintSearch {
    pub policy: BlueprintSearchPolicy,
    pub seed: u64,
}

impl BlueprintSearch {
    /// Generate up to `n` blueprints for the given `problem_spec_hash`.
    ///
    /// For `DeterministicGrid` (and all v0.1 policies, which fall back to it):
    /// vary `quantization_scale` ∈ {4, 5, 6, 7} × `lambda_hard` ∈ {0.5, 1.0}
    /// for 8 unique combinations.  Seeds are `self.seed + index`.
    pub fn generate_blueprints(
        &self,
        problem_spec_hash: &str,
        n: usize,
    ) -> Vec<TraversalBlueprint> {
        // All policies fall back to DeterministicGrid in v0.1.
        self.deterministic_grid(problem_spec_hash, n)
    }

    fn deterministic_grid(
        &self,
        problem_spec_hash: &str,
        n: usize,
    ) -> Vec<TraversalBlueprint> {
        let quantization_scales: &[i32] = &[4, 5, 6, 7];
        let lambda_hards: &[f64] = &[0.5, 1.0];

        let mut blueprints: Vec<TraversalBlueprint> = Vec::new();

        'outer: for &qs in quantization_scales {
            for &lh in lambda_hards {
                if blueprints.len() >= n {
                    break 'outer;
                }
                let index = blueprints.len();
                let seed = self.seed.wrapping_add(index as u64);

                let operator_policy = OperatorBuildConfig {
                    kind: OperatorKind::Laplacian,
                    lambda_hard: lh,
                    lambda_soft: 0.5,
                    quantization_scale: qs,
                    operator_version: "v0.1".into(),
                };

                let signature_policy = SignatureConfig::default();
                let gate_policy = SignatureGateConfig::default();

                let mut bp = TraversalBlueprint {
                    blueprint_id: String::new(),
                    problem_spec_hash: problem_spec_hash.to_string(),
                    topology_policy: TopologyPolicy::Default,
                    collapse_policy: CollapsePolicy::Default,
                    operator_policy,
                    signature_policy,
                    gate_policy,
                    seed,
                    parent_blueprint_id: None,
                };

                bp.blueprint_id = hex_address(&bp).unwrap_or_default();
                blueprints.push(bp);
            }
        }

        blueprints
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_search(seed: u64) -> BlueprintSearch {
        BlueprintSearch {
            policy: BlueprintSearchPolicy::DeterministicGrid,
            seed,
        }
    }

    #[test]
    fn blueprint_id_is_stable() {
        let search = make_search(42);
        let blueprints1 = search.generate_blueprints("hash.abc123", 4);
        let blueprints2 = search.generate_blueprints("hash.abc123", 4);
        assert_eq!(blueprints1.len(), blueprints2.len());
        for (a, b) in blueprints1.iter().zip(blueprints2.iter()) {
            assert_eq!(a.blueprint_id, b.blueprint_id, "blueprint_id must be stable");
            assert_eq!(a.blueprint_id.len(), 64, "blueprint_id must be 64 hex chars");
        }
    }

    #[test]
    fn blueprint_search_is_deterministic_under_seed() {
        let search1 = make_search(7);
        let search2 = make_search(7);
        let v1 = search1.generate_blueprints("spec.hash.xyz", 8);
        let v2 = search2.generate_blueprints("spec.hash.xyz", 8);
        assert_eq!(v1.len(), v2.len());
        for (a, b) in v1.iter().zip(v2.iter()) {
            assert_eq!(a.blueprint_id, b.blueprint_id);
            assert_eq!(a.seed, b.seed);
        }
    }

    #[test]
    fn blueprint_search_different_seeds_produce_different_seeds() {
        let search1 = make_search(0);
        let search2 = make_search(100);
        let v1 = search1.generate_blueprints("hash.same", 2);
        let v2 = search2.generate_blueprints("hash.same", 2);
        // blueprint_ids differ because seed differs
        assert_ne!(v1[0].blueprint_id, v2[0].blueprint_id);
    }

    #[test]
    fn deterministic_grid_produces_at_most_8_blueprints() {
        let search = make_search(0);
        let v = search.generate_blueprints("hash.x", 100);
        assert_eq!(v.len(), 8, "grid has 4×2 = 8 combos");
    }

    #[test]
    fn non_dominated_frontier_removes_dominated() {
        let mut frontier = NonDominatedFrontier::new();
        let mut s1 = BTreeMap::new();
        s1.insert("gap_score".into(), 0.8_f64);
        s1.insert("fragmentation_score".into(), 0.1_f64);
        s1.insert("degeneracy_score".into(), 0.1_f64);
        frontier.add("bp1".into(), s1);

        // Dominated: gap_score lower, rest same
        let mut s2 = BTreeMap::new();
        s2.insert("gap_score".into(), 0.5_f64);
        s2.insert("fragmentation_score".into(), 0.1_f64);
        s2.insert("degeneracy_score".into(), 0.1_f64);
        frontier.add("bp2".into(), s2);

        let ids = frontier.frontier_ids();
        assert_eq!(ids, vec!["bp1".to_string()]);
    }

    #[test]
    fn non_dominated_frontier_keeps_non_dominated_pair() {
        let mut frontier = NonDominatedFrontier::new();
        let mut s1 = BTreeMap::new();
        s1.insert("gap_score".into(), 0.9_f64);
        s1.insert("fragmentation_score".into(), 0.5_f64);
        s1.insert("degeneracy_score".into(), 0.1_f64);
        frontier.add("bp1".into(), s1);

        let mut s2 = BTreeMap::new();
        s2.insert("gap_score".into(), 0.6_f64);
        s2.insert("fragmentation_score".into(), 0.1_f64);
        s2.insert("degeneracy_score".into(), 0.1_f64);
        frontier.add("bp2".into(), s2);

        // Both are non-dominated.
        let ids = frontier.frontier_ids();
        assert_eq!(ids.len(), 2);
    }
}

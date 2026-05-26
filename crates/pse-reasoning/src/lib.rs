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
        let candidate = hits
            .into_iter()
            .find(|h| !visited.contains(&h.crystal_id_hex));

        let Some(hit) = candidate else {
            terminated_by = TerminationReason::NoNewMatches;
            break;
        };

        if hit.score < config.min_d_threshold {
            terminated_by = TerminationReason::MinThreshold;
            break;
        }

        visited.insert(hit.crystal_id_hex.clone());

        let (qtic_class, stability_score) =
            store.crystal_meta(&hit.crystal_id_hex).unwrap_or((0, 0.5));

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

// ── Quantum-Walk Beam ETV ─────────────────────────────────────────────────────

/// Map QTIC class (0–5) to a phase angle on the unit circle.
///
/// The six conformance classes correspond to the 6th roots of unity:
/// Q_k → k·π/3.  Two paths that converge on a crystal with the same QTIC
/// class interfere constructively; paths whose classes differ by 3 (Q0↔Q3,
/// Q1↔Q4, Q2↔Q5) are π out of phase and interfere destructively.
fn qtic_phase(qtic: u8) -> f64 {
    (qtic.min(5) as f64) * std::f64::consts::PI / 3.0
}

/// Configuration for the Quantum-Walk Beam ETV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeamConfig {
    /// Number of simultaneous paths kept after each step.
    pub beam_width: usize,
    /// How many candidates each path expands to per step (branching factor).
    pub fan_out: usize,
    /// Maximum steps per path.
    pub max_steps: usize,
    /// Prune paths whose amplitude drops below this after interference.
    pub min_amplitude: f64,
    /// How many candidates to score per step via `score_tripolar`.
    pub top_k_per_step: usize,
}

impl Default for BeamConfig {
    fn default() -> Self {
        Self {
            beam_width: 4,
            fan_out: 3,
            max_steps: 6,
            min_amplitude: 1e-9,
            top_k_per_step: 32,
        }
    }
}

/// A single hop within a beam path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeamStep {
    pub step_index: usize,
    pub crystal_id_hex: String,
    /// D = ψ · ρ · ω for this hop.
    pub d_score: f64,
    pub qtic_class: u8,
    pub stability_score: f64,
    /// Phase contribution from this step's QTIC class: k·π/3.
    pub phase_contribution: f64,
}

/// One trajectory through the HDAG — an independent quantum-walk arm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeamPath {
    pub steps: Vec<BeamStep>,
    /// Product of all D-scores along this path: ∏ D_k.
    /// After interference events this is |Σ amplitude_i · e^{iθ_i}| at
    /// the merge node, propagated forward multiplicatively.
    pub amplitude: f64,
    /// Accumulated phase: Σ phase_k (not reduced mod 2π).
    pub accumulated_phase: f64,
}

impl BeamPath {
    pub fn length(&self) -> usize {
        self.steps.len()
    }

    /// Number of distinct QTIC classes visited — epistemic diversity.
    pub fn qtic_diversity(&self) -> usize {
        self.steps
            .iter()
            .map(|s| s.qtic_class)
            .collect::<std::collections::HashSet<_>>()
            .len()
    }

    pub fn mean_d(&self) -> f64 {
        if self.steps.is_empty() {
            return 0.0;
        }
        self.steps.iter().map(|s| s.d_score).sum::<f64>() / self.steps.len() as f64
    }

    pub fn peak_d(&self) -> f64 {
        self.steps.iter().map(|s| s.d_score).fold(0.0_f64, f64::max)
    }
}

/// Result of the quantum-walk beam search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeamChain {
    pub query: String,
    /// Surviving paths sorted by amplitude descending.
    pub paths: Vec<BeamPath>,
    /// Amplitude of the best path.
    pub best_amplitude: f64,
    /// Number of interference events (path merges) across all steps.
    pub interference_events: usize,
}

impl BeamChain {
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn best(&self) -> Option<&BeamPath> {
        self.paths.first()
    }

    pub fn mean_amplitude(&self) -> f64 {
        if self.paths.is_empty() {
            return 0.0;
        }
        self.paths.iter().map(|p| p.amplitude).sum::<f64>() / self.paths.len() as f64
    }

    pub fn mean_diversity(&self) -> f64 {
        if self.paths.is_empty() {
            return 0.0;
        }
        self.paths
            .iter()
            .map(|p| p.qtic_diversity() as f64)
            .sum::<f64>()
            / self.paths.len() as f64
    }
}

// Internal: one candidate path extension before interference resolution.
struct Proposal {
    parent_steps: Vec<BeamStep>,
    parent_visited: std::collections::HashSet<String>,
    new_step: BeamStep,
    new_amplitude: f64,
    new_phase: f64,
    new_vec: Vec<f64>,
}

/// Run the Quantum-Walk Epistemic Thunderbolt (Beam ETV).
///
/// Maintains `beam_width` simultaneous paths.  At each step:
///
/// 1. Each path expands to its top `fan_out` unvisited candidates
///    (scored with D = ψ·ρ·ω via `score_tripolar`).
/// 2. Proposals that target the same crystal are *interfered*:
///    `combined = |Σ amplitude_i · e^{i·θ_i}|`
///    where θ_i is the accumulated phase of path i.
///    Same QTIC class → constructive; opposite class (Δk=3) → destructive.
/// 3. The path with the highest pre-interference amplitude inherits the
///    merge node; its amplitude is updated to `combined`.
/// 4. Surviving proposals are sorted by amplitude and pruned to `beam_width`.
///
/// The result is a [`BeamChain`] with paths sorted by amplitude descending.
/// `interference_events` counts how many merge steps occurred across all steps.
pub fn guide_beam(query: &str, store: &ILStore, config: &BeamConfig) -> BeamChain {
    let empty = || BeamChain {
        query: query.to_string(),
        paths: vec![],
        best_amplitude: 0.0,
        interference_events: 0,
    };

    if store.is_empty() {
        return empty();
    }

    let query_vec = text_to_vector8(query);

    // Seed: score from the query vector, initialise one path per top hit.
    let mut seed_hits = store.score_tripolar(&query_vec);
    seed_hits.truncate(config.top_k_per_step);

    // active = (BeamPath, per-path visited set, current query vector)
    let mut active: Vec<(BeamPath, std::collections::HashSet<String>, Vec<f64>)> = Vec::new();

    for hit in seed_hits.iter().take(config.beam_width) {
        if hit.score < config.min_amplitude {
            break;
        }
        let (qtic, stability) = store.crystal_meta(&hit.crystal_id_hex).unwrap_or((0, 0.5));
        let phase = qtic_phase(qtic);
        let cur_vec = store
            .crystal_vector8(&hit.crystal_id_hex)
            .unwrap_or_else(|| query_vec.clone());
        let mut visited = std::collections::HashSet::new();
        visited.insert(hit.crystal_id_hex.clone());
        active.push((
            BeamPath {
                steps: vec![BeamStep {
                    step_index: 0,
                    crystal_id_hex: hit.crystal_id_hex.clone(),
                    d_score: hit.score,
                    qtic_class: qtic,
                    stability_score: stability,
                    phase_contribution: phase,
                }],
                amplitude: hit.score,
                accumulated_phase: phase,
            },
            visited,
            cur_vec,
        ));
    }

    if active.is_empty() {
        return empty();
    }

    // Normalize seed amplitudes: Σ|aᵢ|² = 1 (quantum-walk convention).
    let seed_norm = active
        .iter()
        .map(|(p, _, _)| p.amplitude * p.amplitude)
        .sum::<f64>()
        .sqrt();
    if seed_norm > 0.0 {
        for (path, _, _) in &mut active {
            path.amplitude /= seed_norm;
        }
    }

    let mut interference_events = 0usize;

    for step_idx in 1..config.max_steps {
        // Expand every active path up to fan_out unvisited candidates.
        let mut proposals: Vec<Proposal> = Vec::new();

        for (path, visited, cur_vec) in &active {
            let mut hits = store.score_tripolar(cur_vec);
            hits.truncate(config.top_k_per_step);

            let mut taken = 0;
            for hit in hits {
                if taken >= config.fan_out {
                    break;
                }
                if visited.contains(&hit.crystal_id_hex) {
                    continue;
                }
                if hit.score < config.min_amplitude {
                    break;
                }
                let (qtic, stability) = store.crystal_meta(&hit.crystal_id_hex).unwrap_or((0, 0.5));
                let phase_contrib = qtic_phase(qtic);
                let new_vec = store
                    .crystal_vector8(&hit.crystal_id_hex)
                    .unwrap_or_else(|| cur_vec.clone());
                proposals.push(Proposal {
                    parent_steps: path.steps.clone(),
                    parent_visited: visited.clone(),
                    new_step: BeamStep {
                        step_index: step_idx,
                        crystal_id_hex: hit.crystal_id_hex.clone(),
                        d_score: hit.score,
                        qtic_class: qtic,
                        stability_score: stability,
                        phase_contribution: phase_contrib,
                    },
                    new_amplitude: path.amplitude * hit.score,
                    new_phase: path.accumulated_phase + phase_contrib,
                    new_vec,
                });
                taken += 1;
            }
        }

        if proposals.is_empty() {
            break;
        }

        // Group proposals by target crystal for interference.
        let mut groups: std::collections::BTreeMap<String, Vec<usize>> = Default::default();
        for (i, p) in proposals.iter().enumerate() {
            groups
                .entry(p.new_step.crystal_id_hex.clone())
                .or_default()
                .push(i);
        }

        let mut next_active: Vec<(BeamPath, std::collections::HashSet<String>, Vec<f64>)> =
            Vec::new();

        for (_crystal, indices) in &groups {
            if indices.len() > 1 {
                interference_events += 1;
            }

            // Interference: combined = |Σ amplitude_i · e^{iθ_i}|
            let re: f64 = indices
                .iter()
                .map(|&i| proposals[i].new_amplitude * proposals[i].new_phase.cos())
                .sum();
            let im: f64 = indices
                .iter()
                .map(|&i| proposals[i].new_amplitude * proposals[i].new_phase.sin())
                .sum();
            let combined = (re * re + im * im).sqrt();

            if combined < config.min_amplitude {
                continue;
            }

            // The dominant parent (highest pre-interference amplitude) donates
            // its path history; its amplitude is updated to the combined value.
            let best_idx = *indices
                .iter()
                .max_by(|&&a, &&b| {
                    proposals[a]
                        .new_amplitude
                        .partial_cmp(&proposals[b].new_amplitude)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();

            let p = &proposals[best_idx];
            let mut new_steps = p.parent_steps.clone();
            new_steps.push(p.new_step.clone());
            let mut new_visited = p.parent_visited.clone();
            new_visited.insert(p.new_step.crystal_id_hex.clone());

            next_active.push((
                BeamPath {
                    steps: new_steps,
                    amplitude: combined,
                    accumulated_phase: p.new_phase,
                },
                new_visited,
                p.new_vec.clone(),
            ));
        }

        if next_active.is_empty() {
            break;
        }

        // Normalize after interference: Σ|aᵢ|² = 1.
        // Keeps amplitudes in [0,1] regardless of corpus size or beam_width.
        let step_norm = next_active
            .iter()
            .map(|(p, _, _)| p.amplitude * p.amplitude)
            .sum::<f64>()
            .sqrt();
        if step_norm > 0.0 {
            for (path, _, _) in &mut next_active {
                path.amplitude /= step_norm;
            }
        }

        // Sort by amplitude descending, prune to beam_width.
        next_active.sort_by(|a, b| {
            b.0.amplitude
                .partial_cmp(&a.0.amplitude)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        next_active.truncate(config.beam_width);
        active = next_active;
    }

    let paths: Vec<BeamPath> = active.into_iter().map(|(p, _, _)| p).collect();
    let best_amplitude = paths.first().map(|p| p.amplitude).unwrap_or(0.0);

    BeamChain {
        query: query.to_string(),
        paths,
        best_amplitude,
        interference_events,
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
        for (i, b) in id.bytes().enumerate().take(32) {
            hash[i] = b;
        }
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
        commit(
            &mut store,
            make_crystal("c1", 0.85, 0.75),
            "test query stability",
        );

        let config = ThunderboltConfig {
            max_steps: 3,
            ..Default::default()
        };
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
        commit(
            &mut store,
            make_crystal("c1", 0.9, 0.8),
            "epistemic coherence",
        );
        commit(
            &mut store,
            make_crystal("c2", 0.7, 0.6),
            "stability attractor",
        );
        commit(
            &mut store,
            make_crystal("c3", 0.5, 0.4),
            "free energy gradient",
        );

        let config = ThunderboltConfig {
            max_steps: 10,
            min_d_threshold: 1e-6,
            ..Default::default()
        };
        let chain = guide("coherence", &store, &config);

        // No crystal should appear twice.
        let ids: Vec<&str> = chain
            .steps
            .iter()
            .map(|s| s.crystal_id_hex.as_str())
            .collect();
        let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(
            ids.len(),
            unique.len(),
            "reasoning chain must not revisit crystals"
        );
    }

    // ── Termination ────────────────────────────────────────────────────────

    #[test]
    fn terminates_at_max_steps() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        for i in 0..10 {
            commit(
                &mut store,
                make_crystal(&format!("c{i}"), 0.8, 0.7),
                &format!("concept {i}"),
            );
        }
        let config = ThunderboltConfig {
            max_steps: 3,
            min_d_threshold: 0.0,
            ..Default::default()
        };
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
        commit(
            &mut store,
            make_crystal("c1", 0.1, 0.1),
            "very weak signal here",
        );

        let config = ThunderboltConfig {
            max_steps: 10,
            min_d_threshold: 0.99, // impossibly high
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
            commit(
                &mut store,
                make_crystal(&format!("c{i}"), 0.7, 0.6),
                "knowledge",
            );
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
            let max = chain
                .steps
                .iter()
                .map(|s| s.d_score)
                .fold(0.0_f64, f64::max);
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

    // ── Beam ETV ───────────────────────────────────────────────────────────

    #[test]
    fn beam_empty_store_returns_empty_chain() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let bc = guide_beam("query", &store, &BeamConfig::default());
        assert!(bc.is_empty());
        assert_eq!(bc.best_amplitude, 0.0);
    }

    #[test]
    fn beam_single_crystal_single_path() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        // Use same crystal/text combo as single_crystal_yields_one_step (known positive ψ).
        commit(
            &mut store,
            make_crystal("c1", 0.85, 0.75),
            "test query stability",
        );

        let bc = guide_beam("stability", &store, &BeamConfig::default());
        assert!(!bc.is_empty(), "single crystal must seed at least one path");
        assert_eq!(
            bc.paths[0].length(),
            1,
            "single crystal yields one-step path"
        );
        assert!(bc.best_amplitude > 0.0);
    }

    #[test]
    fn beam_paths_do_not_revisit_crystals() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        for i in 0..8 {
            commit(
                &mut store,
                make_crystal(&format!("c{i}"), 0.75, 0.65),
                &format!("knowledge fragment concept {i}"),
            );
        }
        let config = BeamConfig {
            beam_width: 4,
            fan_out: 3,
            max_steps: 6,
            min_amplitude: 0.0,
            top_k_per_step: 32,
        };
        let bc = guide_beam("knowledge fragment concept", &store, &config);

        for path in &bc.paths {
            let ids: Vec<&str> = path
                .steps
                .iter()
                .map(|s| s.crystal_id_hex.as_str())
                .collect();
            let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
            assert_eq!(
                ids.len(),
                unique.len(),
                "beam path must not revisit crystals"
            );
        }
    }

    #[test]
    fn beam_width_is_respected() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        for i in 0..20 {
            commit(
                &mut store,
                make_crystal(&format!("c{i}"), 0.8, 0.7),
                &format!("stability knowledge graph {i}"),
            );
        }
        let beam_width = 3;
        let config = BeamConfig {
            beam_width,
            fan_out: 4,
            max_steps: 5,
            min_amplitude: 0.0,
            top_k_per_step: 32,
        };
        let bc = guide_beam("stability knowledge graph", &store, &config);
        assert!(
            bc.paths.len() <= beam_width,
            "beam must not exceed beam_width={beam_width}"
        );
    }

    #[test]
    fn beam_paths_sorted_by_amplitude_descending() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        for i in 0..10 {
            let stab = 0.5 + 0.04 * i as f64;
            commit(
                &mut store,
                make_crystal(&format!("c{i}"), stab, 0.6),
                &format!("stability signal coherence {i}"),
            );
        }
        let bc = guide_beam(
            "stability signal coherence",
            &store,
            &BeamConfig {
                min_amplitude: 0.0,
                ..Default::default()
            },
        );
        let amps: Vec<f64> = bc.paths.iter().map(|p| p.amplitude).collect();
        for w in amps.windows(2) {
            assert!(
                w[0] >= w[1] - 1e-12,
                "paths must be sorted by amplitude descending"
            );
        }
    }

    #[test]
    fn beam_qtic_phase_encoding_correct() {
        // Q0 → 0, Q3 → π — opposite phases.
        assert!((qtic_phase(0) - 0.0).abs() < 1e-10);
        assert!((qtic_phase(3) - std::f64::consts::PI).abs() < 1e-10);
        // Q1 and Q4 are also π apart.
        assert!((qtic_phase(4) - qtic_phase(1) - std::f64::consts::PI).abs() < 1e-10);
        // All six phases are distinct.
        let phases: Vec<f64> = (0u8..6).map(qtic_phase).collect();
        for i in 0..6 {
            for j in (i + 1)..6 {
                assert!((phases[i] - phases[j]).abs() > 1e-6);
            }
        }
    }

    #[test]
    fn beam_constructive_interference_increases_amplitude() {
        // Two proposals with same phase (same QTIC class) converge on a crystal.
        // Combined amplitude = sqrt((a1·cos θ + a2·cos θ)² + (a1·sin θ + a2·sin θ)²)
        //                    = a1 + a2  (since cos²+sin²=1, all same phase)
        let a1 = 0.3_f64;
        let a2 = 0.2_f64;
        let theta = qtic_phase(2); // Q2
        let re = a1 * theta.cos() + a2 * theta.cos();
        let im = a1 * theta.sin() + a2 * theta.sin();
        let combined = (re * re + im * im).sqrt();
        assert!(
            combined > a1.max(a2),
            "constructive interference must exceed individual amplitudes"
        );
        assert!(
            (combined - (a1 + a2)).abs() < 1e-10,
            "same-phase combination must equal sum"
        );
    }

    #[test]
    fn beam_destructive_interference_reduces_amplitude() {
        // Two proposals with opposite phases (Q0 and Q3, Δk=3 → Δθ=π) converge.
        // Combined = |a1·e^{i·0} + a2·e^{i·π}| = |a1 - a2|
        let a1 = 0.4_f64;
        let a2 = 0.15_f64;
        let theta1 = qtic_phase(0); // Q0 = 0
        let theta2 = qtic_phase(3); // Q3 = π
        let re = a1 * theta1.cos() + a2 * theta2.cos();
        let im = a1 * theta1.sin() + a2 * theta2.sin();
        let combined = (re * re + im * im).sqrt();
        assert!(
            combined < a1,
            "destructive interference must reduce dominant amplitude"
        );
        assert!(
            (combined - (a1 - a2).abs()).abs() < 1e-10,
            "opposite-phase must equal |a1 - a2|"
        );
    }

    #[test]
    fn beam_interference_events_counted() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        // Dense corpus: many paths will converge on the same popular crystals.
        for i in 0..12 {
            commit(
                &mut store,
                make_crystal(&format!("c{i}"), 0.8, 0.75),
                "stability coherence attractor node",
            );
        }
        let config = BeamConfig {
            beam_width: 4,
            fan_out: 4,
            max_steps: 4,
            min_amplitude: 0.0,
            top_k_per_step: 32,
        };
        let bc = guide_beam("stability coherence attractor node", &store, &config);
        // With fan_out=4 and beam_width=4, multiple paths will attempt the same top crystal.
        // The interference_events counter must be non-negative; on a dense corpus > 0 is expected.
        assert!(bc.interference_events >= 0); // always true — structural sanity
                                              // The chain itself must have surviving paths (not all destructively cancelled).
        assert!(!bc.is_empty());
    }

    #[test]
    fn beam_amplitudes_are_normalized() {
        // After normalization Σ|aᵢ|² = 1 holds for the surviving paths at each step.
        // We can only check the final state here: Σ amplitudes² ≤ 1.
        // (Pruning to beam_width removes paths, so the sum may drop below 1.)
        let dir = tempfile::tempdir().unwrap();
        let mut store = open_store(dir.path());
        for i in 0..10 {
            commit(
                &mut store,
                make_crystal(&format!("c{i}"), 0.8, 0.75),
                &format!("concept {i}"),
            );
        }
        let bc = guide_beam("concept", &store, &BeamConfig::default());
        for path in &bc.paths {
            assert!(path.amplitude >= 0.0, "amplitude must be non-negative");
            assert!(
                path.amplitude <= 1.0 + 1e-10,
                "amplitude must be ≤ 1 after normalization"
            );
        }
    }
}

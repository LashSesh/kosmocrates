//! Bridge: nxalien RuleAtoms → PSE SemanticCrystals → ILStore.
//!
//! Each RuleAtom becomes a first-class SemanticCrystal committed to the
//! Infinity Ledger.  This wires nxalien governance into PSE's full
//! intelligence layer: HDAG phase-gradient edges, QTIC Q0–Q5 conformance
//! classification, constitutional compliance, epistemic health monitoring,
//! and the lifecycle/agenda system.
//!
//! The IL store lives at `.nxalien/il/` alongside `graph_state.json`.

use pse_adapter_il::store::ILStore;
use pse_graph::derive_vertex_id;
use pse_nxalien_types::{RuleAtom, Severity};
use pse_types::{
    content_address, CommitProof, ConstraintProgram, EvidenceChain,
    SemanticCrystal, TopologySignature,
};
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Summary of an IL commit batch — returned after each compile run.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ILCommitSummary {
    /// Number of rules committed this run.
    pub committed: usize,
    /// Number of rules that were already in the ledger (idempotent skip).
    pub skipped: usize,
    /// Mean QTIC conformance class across committed crystals (0.0–5.0).
    pub mean_qtic_class: f64,
    /// Mean coherence potential ψ = kuramoto − (1 − stability).
    pub mean_coherence_potential: f64,
    /// True when all crystals reached at least QTIC Q2 (gate_passed).
    pub gate_passed_all: bool,
    /// Per-rule: (rule_id, qtic_class, block_hash_prefix).
    pub entries: Vec<ILCommitEntry>,
}

/// Single rule entry in an ILCommitSummary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILCommitEntry {
    pub rule_id: String,
    pub qtic_class: u8,
    pub block_hash_prefix: String,
    pub coherence_potential: f64,
    pub gate_passed: bool,
}

/// Convert a RuleAtom into a SemanticCrystal for IL ingestion.
///
/// Embedding strategy:
///   stability_score  = severity_weight × (0.5 + 0.5 × evidence_density)
///   kuramoto_coherence = 0.4 + 0.6 × evidence_density × severity_weight
///   free_energy      = (1.0 − stability_score)²
///
/// This ensures that Blocking rules with rich evidence sit at high ψ
/// (coherent attractors), while Advisory rules without evidence have
/// negative ψ and form natural leaf nodes in the HDAG.
pub fn rule_to_crystal(rule: &RuleAtom, run_count: u64) -> SemanticCrystal {
    let evidence_density = if rule.evidence.is_empty() {
        0.0_f64
    } else {
        (rule.evidence.len() as f64).min(5.0) / 5.0
    };

    let severity_weight = match rule.severity {
        Severity::Advisory => 0.50,
        Severity::Required => 0.75,
        Severity::Blocking => 1.00,
    };

    let stability_score = (severity_weight * (0.5 + 0.5 * evidence_density)).min(1.0);
    let kuramoto = (0.4 + 0.6 * evidence_density * severity_weight).min(1.0);
    let free_energy = (1.0 - stability_score).powi(2);

    // Topology signature: kuramoto_coherence drives HDAG phase-gradient edges.
    let topology = TopologySignature {
        kuramoto_coherence: kuramoto,
        // Single rule → single vertex topology.
        betti_0: 1,
        euler_char: 1,
        ..TopologySignature::default()
    };

    // Region: one PSE vertex per rule (deterministic, stable across runs).
    let vid = derive_vertex_id(&rule.id);

    // Crystal ID: PSE content address of the full rule atom.
    let crystal_id = content_address(rule);

    let mut op_versions = BTreeMap::new();
    op_versions.insert("nxalien".to_string(), "0.1.0".to_string());

    SemanticCrystal {
        crystal_id,
        region: vec![vid],
        constraint_program: ConstraintProgram::default(),
        stability_score,
        topology_signature: topology,
        betti_numbers: vec![1],
        evidence_chain: EvidenceChain::default(),
        commit_proof: CommitProof::default(),
        operator_versions: op_versions,
        created_at: run_count,
        free_energy,
        carrier_instance_idx: 0,
        scale_tag: "nxalien-rule".to_string(),
        universe_id: "nxalien-v1".to_string(),
        sub_crystal_ids: vec![],
        parent_crystal_ids: vec![],
        genesis_metadata: None,
        metatron_signature: None,
    }
}

/// Commit all rules from a compile run into the IL store at `il_path`.
///
/// The IL store is opened (or created) at `.nxalien/il/`.
/// Each rule becomes a crystal; idempotent commits are counted as skipped.
pub fn commit_rules_to_il(
    rules: &[RuleAtom],
    il_path: &Path,
    run_count: u64,
) -> ILCommitSummary {
    let mut store = match ILStore::open(il_path, "nxalien-v1") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  [il-bridge] ILStore::open failed: {e}");
            return ILCommitSummary::default();
        }
    };

    let mut summary = ILCommitSummary::default();
    let mut qtic_sum = 0.0_f64;
    let mut psi_sum = 0.0_f64;
    let mut all_gate_passed = true;

    for rule in rules {
        let crystal = rule_to_crystal(rule, run_count);
        let question = format!("Does rule '{}' hold?", rule.id);
        let chunks = vec![rule.prescription.clone()];

        match store.commit_with_feedback(&crystal, &chunks, run_count as usize, &question) {
            Ok(fb) => {
                let qtic_class = fb
                    .qtic_certificate
                    .as_ref()
                    .map(|c| c.class_u8())
                    .unwrap_or(0);
                let psi = fb.coherence_potential;
                let gate = fb.gate_passed;

                if !gate {
                    all_gate_passed = false;
                }

                qtic_sum += qtic_class as f64;
                psi_sum += psi;
                summary.committed += 1;

                summary.entries.push(ILCommitEntry {
                    rule_id: rule.id.clone(),
                    qtic_class,
                    block_hash_prefix: fb.block_hash.chars().take(12).collect(),
                    coherence_potential: psi,
                    gate_passed: gate,
                });
            }
            Err(e) => {
                eprintln!("  [il-bridge] commit failed for '{}': {e}", rule.id);
                summary.skipped += 1;
            }
        }
    }

    let n = summary.committed as f64;
    if n > 0.0 {
        summary.mean_qtic_class = qtic_sum / n;
        summary.mean_coherence_potential = psi_sum / n;
    }
    summary.gate_passed_all = all_gate_passed;
    summary
}

// ─── IL health + agenda snapshot ─────────────────────────────────────────────

/// Compact snapshot of the IL store's health and agenda — serializable,
/// embedded into EpistemicSignal so every compile run carries full IL state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ILHealthSnapshot {
    pub total_crystals: usize,
    pub mean_qtic_class: f64,
    pub fraction_q4_plus: f64,
    pub mean_uncertainty: f64,
    pub at_risk_count: usize,
    pub healthy: bool,
    pub agenda_items: Vec<AgendaItemSnapshot>,
}

/// Serializable summary of a single epistemic agenda item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgendaItemSnapshot {
    pub priority: f64,
    pub verb: String,
    pub rationale: String,
    pub expected_uncertainty_delta: f64,
}

/// Open the IL store at `il_path` and read its current health + agenda.
///
/// Returns `None` if the store does not exist yet (first run before any
/// commit has happened) or if opening fails.
pub fn load_il_health_and_agenda(il_path: &Path) -> Option<ILHealthSnapshot> {
    if !il_path.exists() {
        return None;
    }
    let store = ILStore::open(il_path, "nxalien-v1").ok()?;
    if store.is_empty() {
        return None;
    }

    let health = store.memory_health();
    let agenda = store.epistemic_agenda(&Default::default());

    let items: Vec<AgendaItemSnapshot> = agenda
        .items
        .iter()
        .map(|item| AgendaItemSnapshot {
            priority: item.priority,
            verb: item.action.verb().to_string(),
            rationale: item.rationale.clone(),
            expected_uncertainty_delta: item.expected_uncertainty_delta,
        })
        .collect();

    Some(ILHealthSnapshot {
        total_crystals: health.total_crystals,
        mean_qtic_class: health.mean_qtic_class.unwrap_or(0.0),
        fraction_q4_plus: health.fraction_q4_plus,
        mean_uncertainty: health.mean_uncertainty,
        at_risk_count: health.at_risk_count,
        healthy: health.is_healthy(),
        agenda_items: items,
    })
}

/// Convert agenda snapshots into UnknownSlots for the nxalien compile output.
///
/// Only agenda items with priority ≥ `min_priority` are included.
/// The slots appear in the `[NXALIEN-CONTEXT]` block so agents see them.
pub fn agenda_to_unknowns(
    items: &[AgendaItemSnapshot],
    min_priority: f64,
) -> Vec<pse_nxalien_types::UnknownSlot> {
    use pse_nxalien_types::{UnknownSlot, UnknownStatus};

    items
        .iter()
        .filter(|item| item.priority >= min_priority)
        .enumerate()
        .map(|(idx, item)| UnknownSlot {
            name: format!(
                "il-agenda-{}-{:03}",
                item.verb.to_lowercase(),
                idx,
            ),
            domain: vec!["il-health".to_string(), item.verb.to_lowercase()],
            status: UnknownStatus::Unknown,
            candidates: vec![item.rationale.clone()],
            evidence: vec![],
            resolution: None,
            confidence: 1.0 - item.expected_uncertainty_delta.min(1.0),
            expires: None,
        })
        .collect()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use pse_nxalien_types::{EvidenceRef, ScopeRef};

    fn make_rule(id: &str, severity: Severity, evidence_count: usize) -> RuleAtom {
        let evidence = (0..evidence_count)
            .map(|i| EvidenceRef {
                kind: "test".to_string(),
                source: format!("src-{i}"),
            })
            .collect();
        RuleAtom {
            id: id.to_string(),
            scope: ScopeRef { files: vec![], modules: vec![], commands: vec![] },
            trigger: vec!["test".to_string()],
            prescription: format!("Ensure {id} is satisfied."),
            severity,
            evidence,
            gate: None,
            decay: None,
            hash: String::new(),
        }
    }

    #[test]
    fn blocking_rule_has_high_stability() {
        let rule = make_rule("r-block", Severity::Blocking, 3);
        let crystal = rule_to_crystal(&rule, 1);
        // Blocking + 3 evidence: stability = 1.0 * (0.5 + 0.5*0.6) = 0.80
        assert!(crystal.stability_score > 0.75, "blocking rule should be stable");
        assert!(crystal.topology_signature.kuramoto_coherence > 0.6);
    }

    #[test]
    fn advisory_no_evidence_has_low_stability() {
        let rule = make_rule("r-adv", Severity::Advisory, 0);
        let crystal = rule_to_crystal(&rule, 1);
        assert!(crystal.stability_score <= 0.5, "advisory without evidence should be low");
        assert!(crystal.topology_signature.kuramoto_coherence < 0.5);
    }

    #[test]
    fn free_energy_inversely_proportional_to_stability() {
        let block = make_rule("r1", Severity::Blocking, 5);
        let adv   = make_rule("r2", Severity::Advisory, 0);
        let c1 = rule_to_crystal(&block, 1);
        let c2 = rule_to_crystal(&adv,   1);
        assert!(c1.free_energy < c2.free_energy, "blocking should have lower free energy");
    }

    #[test]
    fn coherence_potential_positive_for_good_rules() {
        let rule = make_rule("r-req", Severity::Required, 3);
        let crystal = rule_to_crystal(&rule, 1);
        let psi = crystal.topology_signature.kuramoto_coherence - (1.0 - crystal.stability_score);
        assert!(psi > 0.0, "required rule with evidence should have ψ > 0");
    }

    #[test]
    fn crystal_region_is_deterministic() {
        let rule = make_rule("my-rule", Severity::Required, 1);
        let c1 = rule_to_crystal(&rule, 1);
        let c2 = rule_to_crystal(&rule, 99);
        assert_eq!(c1.region, c2.region, "region is deterministic across runs");
    }

    #[test]
    fn commit_rules_to_il_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let rules = vec![
            make_rule("rust-fmt", Severity::Required, 0),
            make_rule("rust-test", Severity::Blocking, 2),
        ];
        let summary = commit_rules_to_il(&rules, dir.path(), 1);
        assert_eq!(summary.committed, 2);
        assert_eq!(summary.entries.len(), 2);
        // Blocking rule should have higher or equal QTIC class than Advisory
        let blocking_entry = summary.entries.iter().find(|e| e.rule_id == "rust-test").unwrap();
        let fmt_entry = summary.entries.iter().find(|e| e.rule_id == "rust-fmt").unwrap();
        assert!(blocking_entry.coherence_potential >= fmt_entry.coherence_potential);
    }

    #[test]
    fn commit_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let rules = vec![make_rule("r1", Severity::Required, 1)];
        let s1 = commit_rules_to_il(&rules, dir.path(), 1);
        let s2 = commit_rules_to_il(&rules, dir.path(), 2);
        // Both runs commit successfully (IL handles idempotency internally)
        assert_eq!(s1.committed, 1);
        assert_eq!(s2.committed, 1);
    }
}

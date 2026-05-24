//! Attractor-constrained rule evolution proposals.
//!
//! When the EpistemicSignal shows Drifting or Diverging stability, this
//! module generates RuleEvolutionProposals — candidate severity adjustments
//! that would move the knowledge field back toward the attractor centroid.
//! Proposals are validated by an EvolutionGuard before application; rejected
//! proposals become UnknownSlots to keep the drift visible.

use crate::signal::{EpistemicSignal, SignalStability};
use pse_nxalien_types::{NxAlienPolicy, RuleAtom, Severity, UnknownSlot, UnknownStatus};

/// A candidate change to a single rule's severity, derived from signal state.
#[derive(Debug, Clone)]
pub struct RuleEvolutionProposal {
    /// ID of the rule to be updated.
    pub rule_id: String,
    /// Current severity of the rule.
    pub current_severity: Severity,
    /// Proposed new severity.
    pub proposed_severity: Severity,
    /// Human-readable rationale referencing signal state.
    pub rationale: String,
    /// Alignment of the proposal with the attractor centroid [0.0, 1.0].
    /// Higher = better aligned; guard rejects proposals below threshold.
    pub attractor_alignment: f64,
}

/// Guard parameters that constrain which proposals may be applied.
///
/// The guard prevents unbounded rule drift: no proposal is applied unless
/// it satisfies ALL guard conditions.
#[derive(Debug, Clone)]
pub struct EvolutionGuard {
    /// Maximum number of severity levels a single rule may be downgraded
    /// in one evolution cycle (0 = no downgrade allowed).
    pub max_severity_downgrade: u8,
    /// If true, proposals that remove evidence requirements are rejected.
    pub require_evidence: bool,
    /// Minimum attractor alignment for a proposal to be accepted [0.0, 1.0].
    pub min_attractor_alignment: f64,
}

impl Default for EvolutionGuard {
    fn default() -> Self {
        Self {
            max_severity_downgrade: 1,
            require_evidence: true,
            min_attractor_alignment: 0.40,
        }
    }
}

/// Generate RuleEvolutionProposals from the current EpistemicSignal.
///
/// Only generates proposals when the signal is Drifting or Diverging.
/// Each proposal targets one rule; severity is adjusted based on how far
/// the embedding has drifted from the attractor centroid.
pub fn propose_rule_evolution(
    signal: &EpistemicSignal,
    rules: &[RuleAtom],
    _policy: &NxAlienPolicy,
) -> Vec<RuleEvolutionProposal> {
    if !signal.needs_rule_review() {
        return vec![];
    }

    let alignment = attractor_alignment_score(signal);
    let severity_pressure = severity_pressure(signal);

    rules
        .iter()
        .filter_map(|rule| propose_for_rule(rule, signal, alignment, severity_pressure))
        .collect()
}

/// Apply guard-validated proposals back to the rule set.
///
/// For each proposal:
///  - If the guard accepts it, the rule's severity is updated in-place.
///  - If the guard rejects it, an UnknownSlot is generated to track the drift.
///
/// Returns the UnknownSlots produced by rejected proposals.
pub fn apply_validated_proposals(
    rules: &mut Vec<RuleAtom>,
    proposals: &[RuleEvolutionProposal],
    guard: &EvolutionGuard,
) -> Vec<UnknownSlot> {
    let mut new_unknowns = Vec::new();

    for proposal in proposals {
        if guard_accepts(guard, proposal) {
            // Apply the severity change in-place.
            if let Some(rule) = rules.iter_mut().find(|r| r.id == proposal.rule_id) {
                rule.severity = proposal.proposed_severity.clone();
                // Recompute hash after mutation.
                let updated = rule.clone().with_hash();
                rule.hash = updated.hash;
            }
        } else {
            // Rejected — surface as an open epistemic question.
            new_unknowns.push(UnknownSlot {
                name: format!("evo-rejected-{}", proposal.rule_id),
                domain: vec!["rule-governance".to_string()],
                status: UnknownStatus::Unknown,
                candidates: vec![format!(
                    "Rule '{}' was proposed for severity change ({:?} → {:?}) but rejected by EvolutionGuard: {}. Attractor alignment: {:.3}",
                    proposal.rule_id,
                    proposal.current_severity,
                    proposal.proposed_severity,
                    proposal.rationale,
                    proposal.attractor_alignment,
                )],
                evidence: vec![],
                resolution: None,
                confidence: proposal.attractor_alignment,
                expires: None,
            });
        }
    }

    new_unknowns
}

// ─── Private helpers ──────────────────────────────────────────────────────────

/// Attractor alignment score: 1.0 when distance is zero, decays to 0.0 at the
/// diverging threshold.  Used as the proposal's alignment field.
fn attractor_alignment_score(signal: &EpistemicSignal) -> f64 {
    const DIVERGING_THRESHOLD: f64 = 0.60;
    let d = signal.distance_to_attractor.min(DIVERGING_THRESHOLD);
    1.0 - (d / DIVERGING_THRESHOLD)
}

/// Severity pressure: how strongly should rules be upgraded?
///
/// Positive = push toward stricter (Diverging signal).
/// Negative = relax (Converging signal — not normally acted on here,
/// but exposed so callers can interpret it).
fn severity_pressure(signal: &EpistemicSignal) -> f64 {
    match signal.stability {
        SignalStability::Diverging => 1.0,
        SignalStability::Drifting => 0.5,
        SignalStability::Converging => -0.3,
        _ => 0.0,
    }
}

/// Build a proposal for a single rule, or None if no change is warranted.
fn propose_for_rule(
    rule: &RuleAtom,
    signal: &EpistemicSignal,
    alignment: f64,
    pressure: f64,
) -> Option<RuleEvolutionProposal> {
    let proposed = target_severity(&rule.severity, pressure)?;

    let rationale = format!(
        "Signal stability {:?} (distance={:.3}, trend={:.4}); attractor alignment={:.3}",
        signal.stability,
        signal.distance_to_attractor,
        signal.free_energy_trend,
        alignment,
    );

    Some(RuleEvolutionProposal {
        rule_id: rule.id.clone(),
        current_severity: rule.severity.clone(),
        proposed_severity: proposed,
        rationale,
        attractor_alignment: alignment,
    })
}

/// Compute target severity given current severity and pressure.
///
/// Returns None if the pressure would produce no change.
fn target_severity(current: &Severity, pressure: f64) -> Option<Severity> {
    // Positive pressure → escalate toward Blocking.
    // No downgrade proposals from this function; downgrades come only from
    // convergence and are controlled entirely by the guard.
    if pressure > 0.3 {
        return match current {
            Severity::Advisory => Some(Severity::Required),
            Severity::Required => Some(Severity::Blocking),
            Severity::Blocking => None, // already at maximum
        };
    }
    None
}

/// True when the EvolutionGuard allows the proposal to be applied.
fn guard_accepts(guard: &EvolutionGuard, proposal: &RuleEvolutionProposal) -> bool {
    // Reject if attractor alignment is too low.
    if proposal.attractor_alignment < guard.min_attractor_alignment {
        return false;
    }

    // Reject if the proposal is a downgrade beyond the allowed level.
    let downgrade_steps = severity_downgrade_steps(&proposal.current_severity, &proposal.proposed_severity);
    if downgrade_steps > guard.max_severity_downgrade as i32 {
        return false;
    }

    // Reject evidence-removing downgrades when require_evidence is set.
    if guard.require_evidence && downgrade_steps > 0 {
        return false;
    }

    true
}

/// Number of steps the severity is being downgraded (negative = upgrade).
fn severity_downgrade_steps(from: &Severity, to: &Severity) -> i32 {
    severity_rank(from) - severity_rank(to)
}

fn severity_rank(s: &Severity) -> i32 {
    match s {
        Severity::Advisory => 0,
        Severity::Required => 1,
        Severity::Blocking => 2,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::{EpistemicSignal, SignalStability};
    use pse_types::{FiveDState, TopologySignature};

    fn make_signal(stability: SignalStability, distance: f64, trend: f64) -> EpistemicSignal {
        EpistemicSignal {
            embedding: FiveDState::default(),
            attractor_centroid: FiveDState::default(),
            distance_to_attractor: distance,
            free_energy_trend: trend,
            run_count: 5,
            topology: TopologySignature::default(),
            stability,
        }
    }

    fn make_rule(id: &str, severity: Severity) -> RuleAtom {
        RuleAtom {
            id: id.to_string(),
            scope: pse_nxalien_types::ScopeRef {
                files: vec![],
                modules: vec![],
                commands: vec![],
            },
            trigger: vec!["test".to_string()],
            prescription: "do something".to_string(),
            severity,
            evidence: vec![],
            gate: None,
            decay: None,
            hash: String::new(),
        }
    }

    #[test]
    fn no_proposals_when_stable() {
        let signal = make_signal(SignalStability::Stable, 0.05, 0.0);
        let rules = vec![make_rule("r1", Severity::Advisory)];
        let policy = NxAlienPolicy::default();
        let proposals = propose_rule_evolution(&signal, &rules, &policy);
        assert!(proposals.is_empty());
    }

    #[test]
    fn proposals_generated_when_drifting() {
        let signal = make_signal(SignalStability::Drifting, 0.3, 0.05);
        let rules = vec![make_rule("r1", Severity::Advisory)];
        let policy = NxAlienPolicy::default();
        let proposals = propose_rule_evolution(&signal, &rules, &policy);
        assert!(!proposals.is_empty());
        assert_eq!(proposals[0].proposed_severity, Severity::Required);
    }

    #[test]
    fn guard_rejects_low_alignment() {
        let guard = EvolutionGuard {
            min_attractor_alignment: 0.80,
            ..Default::default()
        };
        let proposal = RuleEvolutionProposal {
            rule_id: "r1".to_string(),
            current_severity: Severity::Advisory,
            proposed_severity: Severity::Required,
            rationale: "test".to_string(),
            attractor_alignment: 0.50,
        };
        assert!(!guard_accepts(&guard, &proposal));
    }

    #[test]
    fn guard_accepts_upgrade_with_good_alignment() {
        let guard = EvolutionGuard::default();
        let proposal = RuleEvolutionProposal {
            rule_id: "r1".to_string(),
            current_severity: Severity::Advisory,
            proposed_severity: Severity::Required,
            rationale: "test".to_string(),
            attractor_alignment: 0.75,
        };
        assert!(guard_accepts(&guard, &proposal));
    }

    #[test]
    fn rejected_proposals_become_unknowns() {
        let guard = EvolutionGuard {
            min_attractor_alignment: 0.99, // always reject
            ..Default::default()
        };
        let mut rules = vec![make_rule("r1", Severity::Advisory)];
        let proposals = vec![RuleEvolutionProposal {
            rule_id: "r1".to_string(),
            current_severity: Severity::Advisory,
            proposed_severity: Severity::Required,
            rationale: "test".to_string(),
            attractor_alignment: 0.50,
        }];
        let unknowns = apply_validated_proposals(&mut rules, &proposals, &guard);
        assert_eq!(unknowns.len(), 1);
        assert!(unknowns[0].name.contains("r1"));
        // Original rule unchanged
        assert_eq!(rules[0].severity, Severity::Advisory);
    }

    #[test]
    fn accepted_proposals_update_rule_severity() {
        let guard = EvolutionGuard {
            min_attractor_alignment: 0.0,
            require_evidence: false,
            max_severity_downgrade: 0,
        };
        let mut rules = vec![make_rule("r1", Severity::Advisory)];
        let proposals = vec![RuleEvolutionProposal {
            rule_id: "r1".to_string(),
            current_severity: Severity::Advisory,
            proposed_severity: Severity::Required,
            rationale: "test".to_string(),
            attractor_alignment: 0.75,
        }];
        let unknowns = apply_validated_proposals(&mut rules, &proposals, &guard);
        assert!(unknowns.is_empty());
        assert_eq!(rules[0].severity, Severity::Required);
    }

    #[test]
    fn diverging_signal_proposes_maximum_escalation() {
        let signal = make_signal(SignalStability::Diverging, 0.7, 0.2);
        let rules = vec![make_rule("r1", Severity::Required)];
        let policy = NxAlienPolicy::default();
        let proposals = propose_rule_evolution(&signal, &rules, &policy);
        assert!(!proposals.is_empty());
        assert_eq!(proposals[0].proposed_severity, Severity::Blocking);
    }

    #[test]
    fn blocking_rule_generates_no_proposal() {
        let signal = make_signal(SignalStability::Diverging, 0.7, 0.2);
        let rules = vec![make_rule("r1", Severity::Blocking)];
        let policy = NxAlienPolicy::default();
        let proposals = propose_rule_evolution(&signal, &rules, &policy);
        assert!(proposals.is_empty());
    }
}

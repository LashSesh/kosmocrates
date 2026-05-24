//! 8-gate conjunctive evaluation for nxalien rule policy.

use pse_nxalien_types::{GateOutcome, NxAlienGateReport, NxAlienPolicy, RuleAtom, Severity};

/// Auto-downgrade Required rules that lack evidence to Advisory.
///
/// This is applied before gate evaluation so the gate can pass on
/// projects that haven't yet annotated their rules.
pub fn auto_downgrade_rules(rules: &mut [RuleAtom], policy: &NxAlienPolicy) {
    for rule in rules.iter_mut() {
        if rule.severity == Severity::Required
            && rule.evidence.len() < policy.min_evidence_for_required
        {
            rule.severity = Severity::Advisory;
        }
    }
}

/// Evaluate the 8-gate conjunctive policy against the compiled state.
pub fn evaluate_gate(
    rules: &[RuleAtom],
    context_len: usize,
    policy: &NxAlienPolicy,
    replay_ok: bool,
    canon_ok: bool,
) -> NxAlienGateReport {
    let mut notes = Vec::new();

    // G_evidence: every Required/Blocking rule has at least min_evidence_for_required refs.
    let evidence_ok = rules.iter().all(|r| {
        matches!(r.severity, Severity::Advisory)
            || r.evidence.len() >= policy.min_evidence_for_required
    });

    // G_scope: every non-Advisory rule has at least one scope dimension non-empty.
    let scope_ok = rules.iter().all(|r| {
        matches!(r.severity, Severity::Advisory)
            || !r.scope.files.is_empty()
            || !r.scope.modules.is_empty()
            || !r.scope.commands.is_empty()
    });

    // G_delta: always pass (no diff budget configured at this policy level).
    let delta_ok = true;

    // G_budget: total context chars within limit.
    let budget_ok = context_len <= policy.max_context_chars;

    // G_governance: no Blocking rule without evidence.
    let governance_ok = rules
        .iter()
        .all(|r| !matches!(r.severity, Severity::Blocking) || !r.evidence.is_empty());

    // G_bridge: checked by pse-nxalien-pse; here always true.
    let bridge_ok = true;

    if !evidence_ok {
        notes.push(
            "G_evidence: Required/Blocking rules lack evidence — run auto_downgrade_rules first"
                .to_string(),
        );
    }
    if !scope_ok {
        notes.push("G_scope: Some Required/Blocking rules have empty scope".to_string());
    }
    if !replay_ok {
        notes.push("G_replay: Replay hash mismatch".to_string());
    }
    if !canon_ok {
        notes.push("G_canon: Canonical hash mismatch in one or more rules".to_string());
    }
    if !budget_ok {
        notes.push(format!(
            "G_budget: Context {context_len} chars exceeds max {}",
            policy.max_context_chars
        ));
    }
    if !governance_ok {
        notes.push("G_governance: Blocking rule without evidence".to_string());
    }

    let all_pass = evidence_ok
        && scope_ok
        && replay_ok
        && canon_ok
        && delta_ok
        && budget_ok
        && governance_ok
        && bridge_ok;

    let outcome = if all_pass {
        GateOutcome::Accept
    } else if evidence_ok && !replay_ok {
        GateOutcome::Hold
    } else if !evidence_ok && scope_ok && canon_ok {
        GateOutcome::EvidenceOnly
    } else if !governance_ok {
        GateOutcome::Reject
    } else {
        GateOutcome::Hold
    };

    NxAlienGateReport {
        evidence_ok,
        scope_ok,
        replay_ok,
        canon_ok,
        delta_ok,
        budget_ok,
        governance_ok,
        bridge_ok,
        outcome,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pse_nxalien_types::{EvidenceRef, ScopeRef};

    fn blocking_rule_with_evidence() -> RuleAtom {
        RuleAtom {
            id: "test-gate".to_string(),
            scope: ScopeRef {
                files: vec!["**/*.rs".to_string()],
                ..Default::default()
            },
            trigger: vec![],
            prescription: "must pass".to_string(),
            severity: Severity::Blocking,
            evidence: vec![EvidenceRef {
                kind: "file".to_string(),
                source: "Cargo.toml".to_string(),
            }],
            gate: None,
            decay: None,
            hash: String::new(),
        }
    }

    #[test]
    fn all_gates_accept() {
        let rules = vec![blocking_rule_with_evidence()];
        let report = evaluate_gate(&rules, 100, &NxAlienPolicy::default(), true, true);
        assert_eq!(report.outcome, GateOutcome::Accept);
        assert!(report.notes.is_empty());
    }

    #[test]
    fn replay_fail_becomes_hold() {
        let rules = vec![blocking_rule_with_evidence()];
        let report = evaluate_gate(&rules, 100, &NxAlienPolicy::default(), false, true);
        assert_eq!(report.outcome, GateOutcome::Hold);
        assert!(!report.replay_ok);
    }

    #[test]
    fn budget_exceeded() {
        let rules = vec![blocking_rule_with_evidence()];
        let report = evaluate_gate(&rules, 99_999, &NxAlienPolicy::default(), true, true);
        assert!(!report.budget_ok);
    }
}

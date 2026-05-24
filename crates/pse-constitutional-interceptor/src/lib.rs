//! Constitutional Interceptor — evaluates agent actions against nxalien rules.
//!
//! This is the enforcement layer of Pfad B: governance rules are no longer
//! advisory Markdown — they block actions that violate the constitutional
//! framework *before* those actions can touch application state.
//!
//! ## Decision flow
//!
//! ```text
//! ActionContext ─► trigger match? ─► severity ─► Decision
//!
//!   Blocking rule matched  →  Block  (hard stop, 403 on HTTP)
//!   Required rule matched  →  Block  in strict_mode, Warn otherwise
//!   Advisory rule matched  →  Allow  (surfaced in report, never blocks)
//!   No rule matched        →  Allow
//! ```
//!
//! ## Strict mode
//!
//! Strict mode is activated automatically when the EpistemicSignal of the
//! governing PSE instance is `Drifting` or `Diverging`.  In strict mode,
//! Required rules escalate to Block — the governance framework tightens as
//! the system's epistemic health degrades.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use pse_constitutional_interceptor::{ActionContext, ConstitutionalEvaluator};
//!
//! let evaluator = ConstitutionalEvaluator::new(rules);
//! let action = ActionContext::new("run_command", "rm -rf /data", "clean up disk");
//! let report = evaluator.evaluate(&action);
//! if report.decision.is_block() {
//!     eprintln!("Blocked: {:?}", report.decision);
//! }
//! ```

use pse_nxalien_types::{RuleAtom, Severity};
use serde::{Deserialize, Serialize};

// ── Action context ────────────────────────────────────────────────────────────

/// Describes the agent action to be checked against the constitutional rule set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContext {
    /// Verb describing what the agent wants to do (e.g. "edit_file", "run_command").
    pub verb: String,
    /// Target of the action: file path, SQL statement, command string, URL, etc.
    pub target: String,
    /// Free-text description of intent.  Richer text → better trigger coverage.
    pub description: String,
    /// Arbitrary extra metadata (tool name, agent id, repo, session id, …).
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl ActionContext {
    pub fn new(
        verb: impl Into<String>,
        target: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            verb: verb.into(),
            target: target.into(),
            description: description.into(),
            metadata: serde_json::Value::Null,
        }
    }

    /// Flat searchable string: `"<verb> <target> <description>"` lowercased.
    ///
    /// All trigger matching runs against this fingerprint so the caller
    /// doesn't need to worry about casing.
    pub fn fingerprint(&self) -> String {
        format!("{} {} {}", self.verb, self.target, self.description).to_lowercase()
    }
}

// ── Decision ──────────────────────────────────────────────────────────────────

/// Outcome of evaluating an action against the constitutional rule set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub enum Decision {
    /// Action is clear — no rule triggered.
    Allow,
    /// A Blocking (or Required-in-strict-mode) rule was triggered.
    /// The action must be rejected.
    Block {
        rule_id: String,
        reason: String,
    },
    /// A Required rule triggered in non-strict mode.
    /// Action is allowed but the violation is surfaced to the caller.
    Warn {
        rule_id: String,
        reason: String,
    },
}

impl Decision {
    pub fn is_block(&self) -> bool {
        matches!(self, Decision::Block { .. })
    }

    pub fn is_warn(&self) -> bool {
        matches!(self, Decision::Warn { .. })
    }

    pub fn rule_id(&self) -> Option<&str> {
        match self {
            Decision::Block { rule_id, .. } | Decision::Warn { rule_id, .. } => Some(rule_id),
            Decision::Allow => None,
        }
    }
}

// ── Triggered rule ────────────────────────────────────────────────────────────

/// Summary of a single rule that matched the action fingerprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggeredRule {
    pub rule_id: String,
    pub severity: String,
    pub prescription: String,
    pub matched_trigger: String,
}

// ── Evaluation report ─────────────────────────────────────────────────────────

/// Full evaluation result, including all matched rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationReport {
    pub decision: Decision,
    /// Every rule whose triggers matched — not just the first blocker.
    pub triggered_rules: Vec<TriggeredRule>,
    /// Total rules checked (for observability).
    pub checked_rule_count: usize,
    /// Whether strict mode was active during this evaluation.
    pub strict_mode: bool,
}

// ── Evaluator ─────────────────────────────────────────────────────────────────

/// Stateless evaluator: takes a rule set at construction time, evaluates
/// `ActionContext` values on demand.
///
/// The evaluator is `Clone` so it can be shared across Tower service clones
/// without locks.
#[derive(Clone, Default)]
pub struct ConstitutionalEvaluator {
    rules: Vec<RuleAtom>,
    strict_mode: bool,
}

impl ConstitutionalEvaluator {
    /// Build an evaluator from a rule set.  Call `with_strict_mode(true)` to
    /// escalate Required rules to Block.
    pub fn new(rules: Vec<RuleAtom>) -> Self {
        Self { rules, strict_mode: false }
    }

    /// Enable or disable strict mode.  Typically set to `true` when the
    /// governing EpistemicSignal reports `Drifting` or `Diverging`.
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    pub fn is_strict(&self) -> bool {
        self.strict_mode
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Evaluate `action` and return a full `EvaluationReport`.
    pub fn evaluate(&self, action: &ActionContext) -> EvaluationReport {
        let fp = action.fingerprint();
        let mut triggered: Vec<TriggeredRule> = Vec::new();

        for rule in &self.rules {
            if let Some(matched) = first_trigger_match(rule, &fp) {
                triggered.push(TriggeredRule {
                    rule_id: rule.id.clone(),
                    severity: format!("{:?}", rule.severity),
                    prescription: rule.prescription.clone(),
                    matched_trigger: matched,
                });
            }
        }

        let decision = self.derive_decision(&triggered);
        EvaluationReport {
            decision,
            triggered_rules: triggered,
            checked_rule_count: self.rules.len(),
            strict_mode: self.strict_mode,
        }
    }

    fn derive_decision(&self, triggered: &[TriggeredRule]) -> Decision {
        // Pass 1: Blocking rules always win regardless of order in the rule list.
        for t in triggered {
            let sev = self.rules.iter().find(|r| r.id == t.rule_id).map(|r| &r.severity);
            if matches!(sev, Some(Severity::Blocking)) {
                return Decision::Block {
                    rule_id: t.rule_id.clone(),
                    reason: t.prescription.clone(),
                };
            }
        }

        // Pass 2: Required rules — Block in strict mode, Warn otherwise.
        for t in triggered {
            let sev = self.rules.iter().find(|r| r.id == t.rule_id).map(|r| &r.severity);
            if matches!(sev, Some(Severity::Required)) {
                return if self.strict_mode {
                    Decision::Block {
                        rule_id: t.rule_id.clone(),
                        reason: t.prescription.clone(),
                    }
                } else {
                    Decision::Warn {
                        rule_id: t.rule_id.clone(),
                        reason: t.prescription.clone(),
                    }
                };
            }
        }

        Decision::Allow
    }
}

// ── Trigger matching ──────────────────────────────────────────────────────────

fn first_trigger_match(rule: &RuleAtom, fingerprint: &str) -> Option<String> {
    rule.trigger
        .iter()
        .find(|t| fingerprint.contains(t.to_lowercase().as_str()))
        .cloned()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use pse_nxalien_types::{ScopeRef};

    fn rule(id: &str, sev: Severity, triggers: &[&str]) -> RuleAtom {
        RuleAtom {
            id: id.to_string(),
            scope: ScopeRef { files: vec![], modules: vec![], commands: vec![] },
            trigger: triggers.iter().map(|s| s.to_string()).collect(),
            prescription: format!("Ensure {id} is satisfied."),
            severity: sev,
            evidence: vec![],
            gate: None,
            decay: None,
            hash: String::new(),
        }
    }

    fn ev(rules: Vec<RuleAtom>) -> ConstitutionalEvaluator {
        ConstitutionalEvaluator::new(rules)
    }

    // ── Blocking ──────────────────────────────────────────────────────────────

    #[test]
    fn blocking_rule_blocks() {
        let e = ev(vec![rule("no-rm-rf", Severity::Blocking, &["rm -rf"])]);
        let a = ActionContext::new("run_command", "rm -rf /data", "delete everything");
        let r = e.evaluate(&a);
        assert!(r.decision.is_block());
        assert_eq!(r.triggered_rules.len(), 1);
    }

    #[test]
    fn blocking_rule_blocks_on_verb_match() {
        let e = ev(vec![rule("no-drop", Severity::Blocking, &["drop table"])]);
        let a = ActionContext::new("sql", "DROP TABLE users", "remove table");
        let r = e.evaluate(&a);
        assert!(r.decision.is_block(), "trigger matching must be case-insensitive");
    }

    // ── Required ──────────────────────────────────────────────────────────────

    #[test]
    fn required_warns_in_normal_mode() {
        let e = ev(vec![rule("run-tests-before-push", Severity::Required, &["push"])]);
        let a = ActionContext::new("run_command", "git push origin main", "deploy");
        let r = e.evaluate(&a);
        assert!(r.decision.is_warn(), "required must warn, not block, in normal mode");
        assert!(!r.decision.is_block());
    }

    #[test]
    fn required_blocks_in_strict_mode() {
        let e = ev(vec![rule("run-tests", Severity::Required, &["push"])])
            .with_strict_mode(true);
        let a = ActionContext::new("run_command", "git push", "ship it");
        let r = e.evaluate(&a);
        assert!(r.decision.is_block(), "required in strict mode must block");
    }

    // ── Advisory ─────────────────────────────────────────────────────────────

    #[test]
    fn advisory_always_allows_even_in_strict_mode() {
        let e = ev(vec![rule("prefer-fmt", Severity::Advisory, &["fmt", "format"])])
            .with_strict_mode(true);
        let a = ActionContext::new("edit_file", "src/lib.rs", "reformat without cargo fmt");
        let r = e.evaluate(&a);
        assert_eq!(r.decision, Decision::Allow);
    }

    // ── Allow ─────────────────────────────────────────────────────────────────

    #[test]
    fn no_trigger_match_allows() {
        let e = ev(vec![rule("no-secrets", Severity::Blocking, &["password", "secret"])]);
        let a = ActionContext::new("edit_file", "src/config.rs", "update timeout values");
        let r = e.evaluate(&a);
        assert_eq!(r.decision, Decision::Allow);
        assert!(r.triggered_rules.is_empty());
    }

    #[test]
    fn empty_ruleset_always_allows() {
        let e = ev(vec![]);
        let a = ActionContext::new("delete_file", "important.rs", "wipe everything");
        let r = e.evaluate(&a);
        assert_eq!(r.decision, Decision::Allow);
        assert_eq!(r.checked_rule_count, 0);
    }

    // ── Multi-rule ────────────────────────────────────────────────────────────

    #[test]
    fn first_blocking_rule_wins_over_required() {
        let e = ev(vec![
            rule("r-req",   Severity::Required, &["sensitive"]),
            rule("r-block", Severity::Blocking, &["sensitive"]),
        ]);
        let a = ActionContext::new("read_file", ".env", "read sensitive config");
        let r = e.evaluate(&a);
        // Both triggered; first Blocking encountered wins
        assert!(r.decision.is_block());
        assert_eq!(r.triggered_rules.len(), 2);
    }

    #[test]
    fn multiple_advisory_rules_all_allow() {
        let e = ev(vec![
            rule("a1", Severity::Advisory, &["log"]),
            rule("a2", Severity::Advisory, &["debug"]),
        ]);
        let a = ActionContext::new("edit_file", "src/main.rs", "add debug log");
        let r = e.evaluate(&a);
        assert_eq!(r.decision, Decision::Allow);
        assert_eq!(r.triggered_rules.len(), 2);
    }

    // ── Strict mode escalation ─────────────────────────────────────────────

    #[test]
    fn strict_mode_escalation_is_reversible() {
        let rules = vec![rule("run-tests", Severity::Required, &["deploy"])];
        let lax    = ConstitutionalEvaluator::new(rules.clone()).with_strict_mode(false);
        let strict = ConstitutionalEvaluator::new(rules).with_strict_mode(true);
        let a = ActionContext::new("run_command", "deploy to prod", "");

        assert!(lax.evaluate(&a).decision.is_warn());
        assert!(strict.evaluate(&a).decision.is_block());
    }

    // ── Fingerprint ───────────────────────────────────────────────────────────

    #[test]
    fn fingerprint_combines_all_fields() {
        let a = ActionContext::new("EDIT_FILE", "/Sensitive/Path", "Contains SECRET");
        let fp = a.fingerprint();
        assert!(fp.contains("edit_file"));
        assert!(fp.contains("/sensitive/path"));
        assert!(fp.contains("secret"));
    }
}

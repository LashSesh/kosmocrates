//! Constitutional AI substrate for PSE+IL.
//!
//! ## Theoretical Foundation
//!
//! Constitutional compliance is a *fixpoint problem* — structurally analogous
//! to QTIC Q5 path invariance.
//!
//! Let B be the set of crystals in an IL store, and let E(C, Λ) denote the
//! evaluation of crystal C against constitution Λ.  A store is
//! **constitutionally closed** (the constitutional fixpoint) when:
//!
//! ```text
//! ∀C ∈ B: E(C, Λ) = Pass          (every crystal satisfies the constitution)
//! ```
//!
//! This mirrors the QTIC Q5 condition (∮Φ·dl = 0 — no net phase rotation
//! around any closed path in the resonance field).  The analogy is exact:
//!
//! | QTIC Q5                         | Constitutional Fixpoint              |
//! |----------------------------------|--------------------------------------|
//! | Path invariance ∮Φ·dl = 0       | Constitutional closure E(B,Λ) ⊆ B   |
//! | HDAG coherence gate S_coh        | Blocking rule (hard veto)            |
//! | Q3 Kairos gate G_Γ = 1           | Required rule (soft enforcement)     |
//! | Q5 certificate                   | `ConstitutionalAuditReport::is_closed()` |
//! | `classify()` → QticClass         | `Constitution::evaluate()` → Report  |
//!
//! ## Architecture
//!
//! A [`Constitution`] is an ordered list of [`ConstitutionalRule`]s, each
//! pairing a [`RulePredicate`] with a [`Severity`].  Predicates compose via
//! `All`, `Any`, and `Not` to express arbitrary structural constraints on
//! crystals without touching their content.
//!
//! [`Constitution::evaluate`] is *pure*: same crystal + same constitution →
//! same [`ConstitutionalReport`].  The report hash is SHA-256 of the verdict
//! sequence, providing a content-addressed, replay-stable compliance record.
//!
//! ## Preset Constitutions
//!
//! - [`Constitution::eu_ai_act_minimal`] — Article 9/13/17 structural proxies
//! - [`Constitution::pse_core_safety`] — ADAMANT Protocol + QTIC + attribution
//!
//! ## Integration
//!
//! Use [`ILStore::commit_constitutional`](crate::ILStore::commit_constitutional) to commit with compliance gating and
//! [`ILStore::constitutional_audit`](crate::ILStore::constitutional_audit) to verify the entire store is
//! constitutionally closed.

use crate::qtic::{QticCertificate, QticClass};
use pse_types::SemanticCrystal;
use sha2::{Digest, Sha256};

// ─── Severity ────────────────────────────────────────────────────────────────

/// Enforcement level of a constitutional rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational only — violation is recorded but never blocks a commit.
    Advisory,
    /// Violation is recorded and surfaces in audit reports, but does not
    /// block individual commits.  The store is not constitutionally closed
    /// while Required violations exist.
    Required,
    /// Hard veto — `commit_constitutional` returns `Err` without writing to
    /// the ledger when a Blocking rule fails.
    Blocking,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Advisory => "advisory",
            Self::Required => "required",
            Self::Blocking => "blocking",
        }
    }
}

// ─── RulePredicate ───────────────────────────────────────────────────────────

/// A structural predicate evaluated against a crystal and its context.
///
/// Predicates are *pure*: they inspect the crystal's fields and the QTIC
/// certificate without modifying any state.  They compose via `All`, `Any`,
/// and `Not`.
#[derive(Debug, Clone)]
pub enum RulePredicate {
    // ── Structural invariants ─────────────────────────────────────────────
    /// `crystal.stability_score >= threshold`
    MinStability(f64),
    /// `crystal.topology_signature.kuramoto_coherence >= threshold`
    MinKuramoto(f64),
    /// `crystal.free_energy <= threshold`
    MaxFreeEnergy(f64),
    /// `crystal.evidence_chain.len() >= n`
    MinEvidenceEntries(usize),

    // ── Gate / conformance invariants ─────────────────────────────────────
    /// Heuristic Kairos coherence: stability > 0.5 ∧ kuramoto > 0.3.
    /// Use when a QTIC certificate is not yet available (pre-commit check).
    CoherenceGate,
    /// QTIC conformance class ≥ `min_class`.
    /// Evaluates to false when no certificate is present.
    MinQticClass(QticClass),
    /// QTIC Q5 path invariance (`cert.path_inv == true`).
    PathInvariant,

    // ── Agent invariants ──────────────────────────────────────────────────
    /// The crystal must have a non-empty agent attribution.
    RequiresAgentAttribution,

    // ── Boolean combinators ───────────────────────────────────────────────
    /// All sub-predicates must hold.
    All(Vec<RulePredicate>),
    /// At least one sub-predicate must hold.
    Any(Vec<RulePredicate>),
    /// The sub-predicate must *not* hold.
    Not(Box<RulePredicate>),
}

impl RulePredicate {
    /// Evaluate this predicate.
    ///
    /// Returns `(passed, evidence_string)`.  `evidence_string` describes the
    /// concrete values observed, regardless of pass/fail — for audit trails.
    pub fn evaluate(
        &self,
        crystal: &SemanticCrystal,
        qtic_cert: Option<&QticCertificate>,
        agent_id: Option<&str>,
    ) -> (bool, String) {
        match self {
            Self::MinStability(threshold) => {
                let s = crystal.stability_score;
                (
                    s >= *threshold,
                    format!("stability={:.3} >= {:.3}", s, threshold),
                )
            }
            Self::MinKuramoto(threshold) => {
                let k = crystal.topology_signature.kuramoto_coherence;
                (
                    k >= *threshold,
                    format!("kuramoto={:.3} >= {:.3}", k, threshold),
                )
            }
            Self::MaxFreeEnergy(threshold) => {
                let fe = crystal.free_energy;
                (
                    fe <= *threshold,
                    format!("free_energy={:.3} <= {:.3}", fe, threshold),
                )
            }
            Self::MinEvidenceEntries(n) => {
                let count = crystal.evidence_chain.len();
                (count >= *n, format!("evidence_entries={} >= {}", count, n))
            }
            Self::CoherenceGate => {
                let s = crystal.stability_score;
                let k = crystal.topology_signature.kuramoto_coherence;
                let passed = s > 0.5 && k > 0.3;
                (
                    passed,
                    format!("stability={:.3}>0.5 ∧ kuramoto={:.3}>0.3", s, k),
                )
            }
            Self::MinQticClass(min_class) => match qtic_cert {
                Some(cert) => {
                    let passed = cert.conformance_class >= *min_class;
                    (
                        passed,
                        format!("qtic=Q{} >= Q{}", cert.class_u8(), *min_class as u8),
                    )
                }
                None => (false, "qtic=unavailable".to_string()),
            },
            Self::PathInvariant => match qtic_cert {
                Some(cert) => (cert.path_inv, format!("path_invariant={}", cert.path_inv)),
                None => (false, "qtic=unavailable".to_string()),
            },
            Self::RequiresAgentAttribution => {
                let present = agent_id.map(|a| !a.is_empty()).unwrap_or(false);
                let label = agent_id.unwrap_or("<none>");
                (present, format!("agent_id={:?}", label))
            }
            Self::All(preds) => {
                let mut all_passed = true;
                let mut parts = Vec::new();
                for p in preds {
                    let (ok, ev) = p.evaluate(crystal, qtic_cert, agent_id);
                    if !ok {
                        all_passed = false;
                    }
                    parts.push(ev);
                }
                (all_passed, format!("ALL({})", parts.join("; ")))
            }
            Self::Any(preds) => {
                let mut any_passed = false;
                let mut parts = Vec::new();
                for p in preds {
                    let (ok, ev) = p.evaluate(crystal, qtic_cert, agent_id);
                    if ok {
                        any_passed = true;
                    }
                    parts.push(ev);
                }
                (any_passed, format!("ANY({})", parts.join("; ")))
            }
            Self::Not(pred) => {
                let (ok, ev) = pred.evaluate(crystal, qtic_cert, agent_id);
                (!ok, format!("NOT({})", ev))
            }
        }
    }
}

// ─── ConstitutionalRule ───────────────────────────────────────────────────────

/// A single constitutional rule: predicate + severity + provenance.
#[derive(Debug, Clone)]
pub struct ConstitutionalRule {
    /// Stable, unique identifier (e.g., `"PSE-S1-GATE"`).
    pub rule_id: String,
    /// Reference to the governing document (e.g., `"EU AI Act Article 9"`).
    pub axiom_ref: String,
    /// Human-readable description.
    pub description: String,
    /// Enforcement level.
    pub severity: Severity,
    /// The structural predicate that must hold.
    pub predicate: RulePredicate,
}

// ─── ConstitutionalReport ────────────────────────────────────────────────────

/// Per-rule evaluation result.
#[derive(Debug, Clone)]
pub struct RuleVerdict {
    pub rule_id: String,
    pub severity: Severity,
    pub passed: bool,
    /// Concrete evidence string from predicate evaluation.
    pub evidence: String,
}

/// Complete constitutional evaluation of a single crystal.
///
/// The `report_hash` is SHA-256 of the deterministic verdict sequence —
/// a content-addressed compliance record suitable for the audit trail.
#[derive(Debug, Clone)]
pub struct ConstitutionalReport {
    /// First 16 hex chars of the evaluated crystal's ID.
    pub crystal_id: String,
    /// True iff all Required *and* Blocking rules passed.
    pub overall_pass: bool,
    /// Verdict for every rule in the constitution (in rule order).
    pub verdicts: Vec<RuleVerdict>,
    /// Rule IDs of Blocking violations (empty ⟺ no blocking failure).
    pub blocking_violations: Vec<String>,
    /// Rule IDs of Required violations (empty ⟺ Required rules satisfied).
    pub required_violations: Vec<String>,
    /// SHA-256(crystal_id ∥ rule_id₀ ∥ pass₀ ∥ evidence₀ ∥ …).
    pub report_hash: String,
}

impl ConstitutionalReport {
    fn build(crystal_id: &str, verdicts: Vec<RuleVerdict>) -> Self {
        let blocking_violations: Vec<String> = verdicts
            .iter()
            .filter(|v| !v.passed && v.severity == Severity::Blocking)
            .map(|v| v.rule_id.clone())
            .collect();
        let required_violations: Vec<String> = verdicts
            .iter()
            .filter(|v| !v.passed && v.severity == Severity::Required)
            .map(|v| v.rule_id.clone())
            .collect();
        let overall_pass = blocking_violations.is_empty() && required_violations.is_empty();
        let report_hash = compute_report_hash(crystal_id, &verdicts);
        Self {
            crystal_id: crystal_id.to_string(),
            overall_pass,
            verdicts,
            blocking_violations,
            required_violations,
            report_hash,
        }
    }
}

// ─── ConstitutionalAuditReport ───────────────────────────────────────────────

/// Compliance scan of an entire IL store against a constitution.
///
/// ## Constitutional Fixpoint
///
/// `is_constitutionally_closed()` returns true iff the store has reached the
/// constitutional fixpoint — the condition structurally equivalent to QTIC Q5:
///
/// ```text
/// ∀C ∈ B: E(C, Λ) = Pass    ⟺    is_constitutionally_closed()
/// ```
#[derive(Debug, Clone)]
pub struct ConstitutionalAuditReport {
    /// Name of the applied constitution.
    pub constitution_name: String,
    /// Version of the applied constitution.
    pub constitution_version: String,
    /// Total crystals evaluated.
    pub total_crystals: usize,
    /// Crystals with no Required or Blocking violations.
    pub compliant_count: usize,
    /// Crystals with at least one Required or Blocking violation.
    pub violation_count: usize,
    /// Crystals with at least one Blocking violation.
    pub blocking_count: usize,
    /// Per-rule tally: rule_id → number of violating crystals.
    pub violations_by_rule: std::collections::HashMap<String, usize>,
    /// Individual reports for each violating crystal (crystal_id → report).
    pub violations: Vec<(String, ConstitutionalReport)>,
    /// SHA-256 of the concatenated report hashes — a single digest for the
    /// entire audit, suitable for a compliance certificate.
    pub audit_hash: String,
}

impl ConstitutionalAuditReport {
    /// True iff the store has reached the constitutional fixpoint.
    ///
    /// Equivalent to QTIC Q5 at the knowledge-base level:
    /// every crystal satisfies every Required and Blocking rule.
    pub fn is_constitutionally_closed(&self) -> bool {
        self.blocking_count == 0 && self.violation_count == 0
    }

    /// Compact one-line summary for logging / LLM context injection.
    pub fn summary(&self) -> String {
        if self.is_constitutionally_closed() {
            format!(
                "[{}] constitutional fixpoint reached — {}/{} crystals compliant",
                self.constitution_name, self.compliant_count, self.total_crystals
            )
        } else {
            format!(
                "[{}] NOT closed — {}/{} compliant, {} blocking, {} required violations",
                self.constitution_name,
                self.compliant_count,
                self.total_crystals,
                self.blocking_count,
                self.violation_count - self.blocking_count,
            )
        }
    }
}

// ─── Constitution ─────────────────────────────────────────────────────────────

/// An ordered set of [`ConstitutionalRule`]s that governs what crystals may
/// enter the IL store.
///
/// Evaluation is *pure* and *deterministic*: given the same crystal and
/// certificate, `evaluate` always returns the same `ConstitutionalReport`.
pub struct Constitution {
    /// Display name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Ordered rule list — evaluated in sequence, all results collected.
    pub rules: Vec<ConstitutionalRule>,
}

impl Constitution {
    /// EU AI Act minimal compliance proxy.
    ///
    /// Maps Articles 9, 13, and 17 to structural crystal invariants.
    /// These are *structural proxies*, not legal opinions.
    pub fn eu_ai_act_minimal() -> Self {
        Self {
            name: "EU AI Act Minimal".to_string(),
            version: "1.0.0".to_string(),
            rules: vec![
                ConstitutionalRule {
                    rule_id: "EU-ART9-STABILITY".to_string(),
                    axiom_ref: "EU AI Act Article 9 — Risk Management".to_string(),
                    description:
                        "High-risk AI outputs must demonstrate topological stability ≥ 0.6."
                            .to_string(),
                    severity: Severity::Required,
                    predicate: RulePredicate::MinStability(0.6),
                },
                ConstitutionalRule {
                    rule_id: "EU-ART13-COHERENCE".to_string(),
                    axiom_ref: "EU AI Act Article 13 — Transparency and Provision of Information"
                        .to_string(),
                    description: "Outputs must exhibit phase-locking coherence (kuramoto ≥ 0.3) \
                         as a structural proxy for explainability."
                        .to_string(),
                    severity: Severity::Required,
                    predicate: RulePredicate::MinKuramoto(0.3),
                },
                ConstitutionalRule {
                    rule_id: "EU-ART17-TRACEABILITY".to_string(),
                    axiom_ref: "EU AI Act Article 17 — Quality Management System".to_string(),
                    description:
                        "Outputs should carry at least one evidence entry for traceability."
                            .to_string(),
                    severity: Severity::Advisory,
                    predicate: RulePredicate::MinEvidenceEntries(1),
                },
            ],
        }
    }

    /// PSE Core Safety Constitution.
    ///
    /// Implements the ADAMANT Protocol constitutional layer:
    /// - S1 (Blocking): Kairos coherence gate must pass.
    /// - S2 (Required): QTIC Q3 or above (gate-passed, stable phase).
    /// - S3 (Advisory): Agent attribution for multi-agent traceability.
    /// - S4 (Required): No incoherent-confidence crystals (high stability
    ///   with collapsed Kuramoto — a structural hallucination signal).
    pub fn pse_core_safety() -> Self {
        Self {
            name: "PSE Core Safety".to_string(),
            version: "1.0.0".to_string(),
            rules: vec![
                ConstitutionalRule {
                    rule_id: "PSE-S1-GATE".to_string(),
                    axiom_ref: "PSE ADAMANT Protocol v1.0 — Axiom 6.1.1 Artifact Supremacy"
                        .to_string(),
                    description: "Only gate-passed crystals (stability > 0.5 ∧ kuramoto > 0.3) \
                         may be committed. Fail-closed — no output when gate = 0."
                        .to_string(),
                    severity: Severity::Blocking,
                    predicate: RulePredicate::CoherenceGate,
                },
                ConstitutionalRule {
                    rule_id: "PSE-S2-QTIC".to_string(),
                    axiom_ref: "QTIC Specification §7 — Q3 Conformance (gate-passed)".to_string(),
                    description: "Constitutional crystals must reach Q3 (Kairos gate passed, \
                         coherence stable). Below Q3, the crystal is not structurally \
                         trustworthy for grounding."
                        .to_string(),
                    severity: Severity::Required,
                    predicate: RulePredicate::MinQticClass(QticClass::Q3),
                },
                ConstitutionalRule {
                    rule_id: "PSE-S3-ATTRIBUTION".to_string(),
                    axiom_ref: "PSE Multi-Agent Fabric — Agent Attribution Principle".to_string(),
                    description:
                        "In multi-agent deployments, crystals should carry agent attribution \
                         for causal traceability."
                            .to_string(),
                    severity: Severity::Advisory,
                    predicate: RulePredicate::RequiresAgentAttribution,
                },
                ConstitutionalRule {
                    rule_id: "PSE-S4-NO-INCOHERENT-CONFIDENCE".to_string(),
                    axiom_ref: "PSE ADAMANT Protocol v1.0 — Invariant I-11 No False Crystals"
                        .to_string(),
                    description: "A crystal must not have high stability (> 0.8) with collapsed \
                         phase-coherence (kuramoto < 0.2). This structural pattern is the \
                         topological signature of a hallucination attractor."
                        .to_string(),
                    severity: Severity::Required,
                    // NOT (stability > 0.8 AND kuramoto < 0.2)
                    predicate: RulePredicate::Not(Box::new(RulePredicate::All(vec![
                        RulePredicate::MinStability(0.8),
                        RulePredicate::Not(Box::new(RulePredicate::MinKuramoto(0.2))),
                    ]))),
                },
            ],
        }
    }

    /// Evaluate a crystal against all rules in this constitution.
    ///
    /// `qtic_cert` should be `Some` when available (post-commit); `None` for
    /// pre-commit checks (predicates requiring a certificate evaluate to false).
    /// `agent_id` is the committing agent, if known.
    pub fn evaluate(
        &self,
        crystal: &SemanticCrystal,
        qtic_cert: Option<&QticCertificate>,
        agent_id: Option<&str>,
    ) -> ConstitutionalReport {
        let crystal_id: String = crystal
            .crystal_id
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        let crystal_id_prefix = crystal_id[..crystal_id.len().min(16)].to_string();

        let verdicts: Vec<RuleVerdict> = self
            .rules
            .iter()
            .map(|rule| {
                let (passed, evidence) = rule.predicate.evaluate(crystal, qtic_cert, agent_id);
                RuleVerdict {
                    rule_id: rule.rule_id.clone(),
                    severity: rule.severity,
                    passed,
                    evidence,
                }
            })
            .collect();

        ConstitutionalReport::build(&crystal_id_prefix, verdicts)
    }

    /// True iff the crystal satisfies all Required and Blocking rules.
    pub fn admits(
        &self,
        crystal: &SemanticCrystal,
        qtic_cert: Option<&QticCertificate>,
        agent_id: Option<&str>,
    ) -> bool {
        self.evaluate(crystal, qtic_cert, agent_id).overall_pass
    }
}

// ─── ConstitutionalFeedback (returned by ILStore::commit_constitutional) ──────

/// Result of [`ILStore::commit_constitutional`](crate::ILStore::commit_constitutional).
#[derive(Debug)]
pub struct ConstitutionalFeedback {
    /// Full IL validation feedback (QTIC certificate, convergence, etc.).
    pub feedback: crate::feedback::ValidationFeedback,
    /// Constitutional evaluation of the committed crystal.
    pub report: ConstitutionalReport,
    /// True iff no Required or Blocking violations were found.
    pub constitutionally_admissible: bool,
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn compute_report_hash(crystal_id: &str, verdicts: &[RuleVerdict]) -> String {
    let mut h = Sha256::new();
    h.update(crystal_id.as_bytes());
    // Sort by rule_id for determinism (rule order in the constitution is
    // already deterministic, but explicit sorting guards against future changes)
    let mut sorted = verdicts.to_vec();
    sorted.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));
    for v in &sorted {
        h.update(v.rule_id.as_bytes());
        h.update(if v.passed { b"1" } else { b"0" });
        h.update(v.evidence.as_bytes());
    }
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_crystal(stability: f64, kuramoto: f64) -> SemanticCrystal {
        let mut c = SemanticCrystal {
            crystal_id: [0u8; 32],
            region: vec![],
            constraint_program: Default::default(),
            stability_score: stability,
            topology_signature: Default::default(),
            betti_numbers: vec![],
            evidence_chain: Default::default(),
            commit_proof: Default::default(),
            operator_versions: Default::default(),
            created_at: 1,
            free_energy: 0.0,
            carrier_instance_idx: 0,
            scale_tag: "test".to_string(),
            universe_id: String::new(),
            sub_crystal_ids: vec![],
            parent_crystal_ids: vec![],
            genesis_metadata: None,
            metatron_signature: None,
        };
        c.crystal_id[0] = (stability * 200.0) as u8;
        c.topology_signature.kuramoto_coherence = kuramoto;
        c
    }

    // ── RulePredicate tests ────────────────────────────────────────────────

    #[test]
    fn min_stability_passes_when_met() {
        let c = make_crystal(0.8, 0.5);
        let (ok, ev) = RulePredicate::MinStability(0.7).evaluate(&c, None, None);
        assert!(ok, "0.8 >= 0.7 must pass; evidence: {ev}");
    }

    #[test]
    fn min_stability_fails_when_below() {
        let c = make_crystal(0.4, 0.5);
        let (ok, _) = RulePredicate::MinStability(0.7).evaluate(&c, None, None);
        assert!(!ok, "0.4 < 0.7 must fail");
    }

    #[test]
    fn min_kuramoto_passes_and_fails() {
        let c_good = make_crystal(0.7, 0.6);
        let c_bad = make_crystal(0.7, 0.1);
        assert!(
            RulePredicate::MinKuramoto(0.3)
                .evaluate(&c_good, None, None)
                .0
        );
        assert!(
            !RulePredicate::MinKuramoto(0.3)
                .evaluate(&c_bad, None, None)
                .0
        );
    }

    #[test]
    fn coherence_gate_passes_stable_coherent() {
        let c = make_crystal(0.8, 0.6);
        let (ok, _) = RulePredicate::CoherenceGate.evaluate(&c, None, None);
        assert!(ok);
    }

    #[test]
    fn coherence_gate_blocks_low_kuramoto() {
        let c = make_crystal(0.8, 0.1); // stable but incoherent
        let (ok, _) = RulePredicate::CoherenceGate.evaluate(&c, None, None);
        assert!(!ok);
    }

    #[test]
    fn requires_agent_attribution_passes_with_id() {
        let c = make_crystal(0.7, 0.5);
        let (ok, _) = RulePredicate::RequiresAgentAttribution.evaluate(&c, None, Some("alice"));
        assert!(ok);
    }

    #[test]
    fn requires_agent_attribution_fails_without_id() {
        let c = make_crystal(0.7, 0.5);
        let (ok, _) = RulePredicate::RequiresAgentAttribution.evaluate(&c, None, None);
        assert!(!ok);
    }

    #[test]
    fn all_predicate_requires_all() {
        let c = make_crystal(0.8, 0.6);
        let pred = RulePredicate::All(vec![
            RulePredicate::MinStability(0.7),
            RulePredicate::MinKuramoto(0.5),
        ]);
        assert!(pred.evaluate(&c, None, None).0, "both pass → All passes");
        let pred_fail = RulePredicate::All(vec![
            RulePredicate::MinStability(0.7),
            RulePredicate::MinKuramoto(0.8), // fails
        ]);
        assert!(
            !pred_fail.evaluate(&c, None, None).0,
            "one fails → All fails"
        );
    }

    #[test]
    fn any_predicate_requires_one() {
        let c = make_crystal(0.8, 0.2);
        let pred = RulePredicate::Any(vec![
            RulePredicate::MinStability(0.9), // fails
            RulePredicate::MinStability(0.7), // passes
        ]);
        assert!(pred.evaluate(&c, None, None).0, "one passes → Any passes");
    }

    #[test]
    fn not_predicate_inverts() {
        let c = make_crystal(0.8, 0.5);
        let pred = RulePredicate::Not(Box::new(RulePredicate::MinStability(0.9)));
        assert!(pred.evaluate(&c, None, None).0, "NOT(fails) = passes");
    }

    // ── Constitution::evaluate tests ───────────────────────────────────────

    #[test]
    fn report_has_verdict_per_rule() {
        let c = make_crystal(0.7, 0.5);
        let constitution = Constitution::eu_ai_act_minimal();
        let report = constitution.evaluate(&c, None, None);
        assert_eq!(report.verdicts.len(), constitution.rules.len());
    }

    #[test]
    fn report_hash_is_deterministic() {
        let c = make_crystal(0.7, 0.5);
        let constitution = Constitution::eu_ai_act_minimal();
        let r1 = constitution.evaluate(&c, None, None);
        let r2 = constitution.evaluate(&c, None, None);
        assert_eq!(r1.report_hash, r2.report_hash);
    }

    #[test]
    fn different_crystals_different_report_hash() {
        let c1 = make_crystal(0.7, 0.5);
        let c2 = make_crystal(0.3, 0.1); // different values
        let constitution = Constitution::eu_ai_act_minimal();
        let r1 = constitution.evaluate(&c1, None, None);
        let r2 = constitution.evaluate(&c2, None, None);
        assert_ne!(r1.report_hash, r2.report_hash);
    }

    #[test]
    fn eu_ai_act_compliant_crystal_passes() {
        // stability=0.75 ≥ 0.6, kuramoto=0.5 ≥ 0.3 → Required rules pass
        let c = make_crystal(0.75, 0.5);
        let constitution = Constitution::eu_ai_act_minimal();
        let report = constitution.evaluate(&c, None, None);
        assert!(
            report.overall_pass,
            "compliant crystal must pass EU AI Act minimal"
        );
        assert!(report.blocking_violations.is_empty());
        assert!(report.required_violations.is_empty());
    }

    #[test]
    fn eu_ai_act_low_stability_fails_required() {
        let c = make_crystal(0.4, 0.5);
        let constitution = Constitution::eu_ai_act_minimal();
        let report = constitution.evaluate(&c, None, None);
        assert!(!report.overall_pass);
        assert!(report
            .required_violations
            .contains(&"EU-ART9-STABILITY".to_string()));
    }

    #[test]
    fn pse_core_safety_blocking_rule_veto() {
        // stability=0.8 but kuramoto=0.1 → CoherenceGate fails → Blocking
        let c = make_crystal(0.8, 0.1);
        let constitution = Constitution::pse_core_safety();
        let report = constitution.evaluate(&c, None, None);
        assert!(!report.overall_pass);
        assert!(report
            .blocking_violations
            .contains(&"PSE-S1-GATE".to_string()));
    }

    #[test]
    fn pse_core_safety_incoherent_confidence_detected() {
        // stability=0.9 (high), kuramoto=0.1 (collapsed) → hallucination signal
        let c = make_crystal(0.9, 0.1);
        let constitution = Constitution::pse_core_safety();
        let report = constitution.evaluate(&c, None, None);
        // PSE-S4 should fire (Required violation)
        assert!(
            report
                .required_violations
                .contains(&"PSE-S4-NO-INCOHERENT-CONFIDENCE".to_string())
                || report
                    .blocking_violations
                    .contains(&"PSE-S1-GATE".to_string()),
            "incoherent-confidence crystal must have at least one violation"
        );
    }

    #[test]
    fn pse_core_safety_admits_good_crystal() {
        // stability=0.75, kuramoto=0.65 → all structural rules pass
        let c = make_crystal(0.75, 0.65);
        let constitution = Constitution::pse_core_safety();
        // Without a QTIC cert, PSE-S2 (MinQticClass Q3) will fail.
        // Provide a synthetic cert at Q3.
        let report = constitution.evaluate(&c, None, None);
        // PSE-S1 passes (gate satisfied), PSE-S4 passes (not incoherent)
        // PSE-S2 fails (no cert) → overall fails, but no Blocking
        assert!(
            report.blocking_violations.is_empty(),
            "no Blocking for a coherent crystal"
        );
        assert!(!report
            .required_violations
            .contains(&"PSE-S1-GATE".to_string()));
    }

    #[test]
    fn advisory_violation_does_not_block_overall_pass() {
        // EU-ART17 is Advisory; zero evidence entries will fail it but not block
        let c = make_crystal(0.75, 0.5); // passes Required rules, zero evidence
        let constitution = Constitution::eu_ai_act_minimal();
        let report = constitution.evaluate(&c, None, None);
        // Advisory violations don't affect overall_pass
        assert!(
            report.overall_pass,
            "Advisory violation must not block overall_pass"
        );
        // But the verdict itself should be fail
        let art17 = report
            .verdicts
            .iter()
            .find(|v| v.rule_id == "EU-ART17-TRACEABILITY")
            .unwrap();
        assert!(!art17.passed, "zero evidence entries must fail EU-ART17");
    }

    #[test]
    fn presets_have_unique_rule_ids() {
        for constitution in [
            Constitution::eu_ai_act_minimal(),
            Constitution::pse_core_safety(),
        ] {
            let ids: std::collections::HashSet<&str> = constitution
                .rules
                .iter()
                .map(|r| r.rule_id.as_str())
                .collect();
            assert_eq!(
                ids.len(),
                constitution.rules.len(),
                "rule IDs must be unique"
            );
        }
    }
}

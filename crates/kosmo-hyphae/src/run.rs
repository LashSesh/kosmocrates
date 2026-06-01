use crate::assimilation::{AssimilationDecision, AssimilationLedger, NegativeEvidenceRecord};
use crate::frontier::{SourceFrontierGraph, SourceIntent};
use crate::gates::GateCascade;
use crate::host::HostCube;
use crate::structural_yield::{StructuralYield, StructuralYieldKind};
use kosmo_core::{
    AuthorityLabel, Digest, EvidenceBundle, EvidenceKind, EvidenceRef,
    PolicyProfile, ReplayStatus, TaintLabel,
};
use kosmo_workbench::workspace::WorkspaceIndex;
use serde::{Deserialize, Serialize};

/// Serialize-only for run-id content-addressing.
#[derive(Serialize)]
struct RunContent {
    host_cube_id: Digest,
    frontier_id: Digest,
    decision_count: u64,
    ledger_id: Digest,
    policy_id: Digest,
}

/// The result of a HYPHAE v0.3 passive analysis run.
///
/// No host file writes occur. All results are observations and decisions only.
/// Mode is passive/ReportOnly; `accepted_count` will be zero unless
/// a `PolicyProfile` with elevated permissions is used.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyphaeRunResult {
    pub run_id: Digest,
    pub host_cube: HostCube,
    pub frontier: SourceFrontierGraph,
    pub decisions: Vec<AssimilationDecision>,
    pub negative_evidence: Vec<NegativeEvidenceRecord>,
    /// Sequenced, content-addressed audit log of all decisions (INVARIANT-007).
    /// `ledger_id` changes if any decision outcome changes across runs.
    pub ledger: AssimilationLedger,
    pub accepted_count: usize,
    pub rejected_count: usize,
    pub evidence_only_count: usize,
    pub policy_id: Digest,
}

impl HyphaeRunResult {
    pub fn is_clean(&self) -> bool {
        self.rejected_count == 0
    }

    pub fn total_yields(&self) -> usize {
        self.decisions.len()
    }

    /// Human-readable passive-run summary for operator review.
    pub fn summary(&self) -> String {
        format!(
            "HYPHAE v0.3 passive run — policy: {} | voids: {} | \
             yields: {} | accepted: {} | rejected: {} | evidence-only: {} | \
             negative-evidence records: {}",
            self.policy_id.to_hex(),
            self.host_cube.void_count(),
            self.total_yields(),
            self.accepted_count,
            self.rejected_count,
            self.evidence_only_count,
            self.negative_evidence.len(),
        )
    }
}

/// Run a HYPHAE v0.3 passive analysis on a workspace index.
///
/// This is the main entry point for Phase 3. It does not mutate the host
/// workspace. Returns a `HyphaeRunResult` with void map, deficiency vector,
/// gate traces, and assimilation decisions.
pub fn passive_run(index: &WorkspaceIndex, policy: &PolicyProfile) -> HyphaeRunResult {
    passive_run_augmented(index, policy, vec![])
}

/// Run a HYPHAE v0.3 passive analysis with additional intents injected into the frontier.
///
/// Used by the pipeline to inject `SuggestPattern` intents from prior-run
/// `MotifCandidate` objects, closing the motif feedback loop across pipeline runs.
/// The base frontier (from void map) is built first; additional intents are appended
/// before gate processing. All results appear in the returned `HyphaeRunResult`.
pub fn passive_run_augmented(
    index: &WorkspaceIndex,
    policy: &PolicyProfile,
    additional_intents: Vec<SourceIntent>,
) -> HyphaeRunResult {
    let host_cube = HostCube::from_workspace_index(index, policy);
    let base = SourceFrontierGraph::from_void_map(&host_cube.void_map, policy.id);

    let frontier = if additional_intents.is_empty() {
        base
    } else {
        let mut all_intents = base.intents;
        all_intents.extend(additional_intents);
        SourceFrontierGraph::from_intents(all_intents, policy.id)
    };

    let cascade = GateCascade::standard_gates(policy.clone());

    let mut decisions = Vec::new();
    let mut negative_evidence = Vec::new();

    for intent in &frontier.intents {
        let evidence = synthetic_evidence_for_intent(intent, policy.id);
        let yield_ = yield_for_intent(intent, &evidence, policy.id);
        let trace = cascade.apply(&yield_, &evidence);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &evidence, policy.id);

        if let Some(neg) = NegativeEvidenceRecord::from_decision(&decision) {
            negative_evidence.push(neg);
        }

        decisions.push(decision);
    }

    let accepted_count = decisions.iter().filter(|d| d.outcome.is_accepted()).count();
    let rejected_count = decisions.iter().filter(|d| d.outcome.is_rejected()).count();
    let evidence_only_count = decisions.len() - accepted_count - rejected_count;

    // Build the ledger before sealing run_id so ledger_id participates in it.
    // Two runs with identical intents but different gate outcomes will have different run_ids.
    let ledger_placeholder_run_id = Digest::of_bytes(b"ledger-pre-seal");
    let ledger = AssimilationLedger::from_decisions(&decisions, ledger_placeholder_run_id, policy.id);

    let run_id = Digest::of(&RunContent {
        host_cube_id: host_cube.cube_id,
        frontier_id: frontier.graph_id,
        decision_count: decisions.len() as u64,
        ledger_id: ledger.ledger_id,
        policy_id: policy.id,
    });

    // Re-seal ledger with the final run_id.
    let ledger = AssimilationLedger::from_decisions(&decisions, run_id, policy.id);

    HyphaeRunResult {
        run_id,
        host_cube,
        frontier,
        decisions,
        negative_evidence,
        ledger,
        accepted_count,
        rejected_count,
        evidence_only_count,
        policy_id: policy.id,
    }
}

/// Construct a minimal synthetic evidence bundle for one intent.
///
/// Phase 3 evidence is host-scan-only (no external acquisition).
fn synthetic_evidence_for_intent(intent: &SourceIntent, policy_id: Digest) -> EvidenceBundle {
    let ev_ref = EvidenceRef::new(
        intent.intent_id,
        EvidenceKind::HostScan,
        "hyphae-passive-scan",
    );
    EvidenceBundle::seal(vec![ev_ref], policy_id, ReplayStatus::Replayable)
}

/// Build a `StructuralYield` from a `SourceIntent`, propagating its taint and authority.
///
/// The yield's trust level is derived entirely from the intent — no hardcoded
/// overrides. A `TaintLabel::Clean` + `AuthorityLabel::Foundry` intent produces
/// a yield that passes all gates and yields an `Accepted` decision.
///
/// Yield kind is selected from intent kind:
/// - `FillVoid` / `ReduceDeficiency` → `DeficiencyFill`
/// - `SuggestPattern` → `MotifProposal`
/// - `Custom` → `DeficiencyFill` (safe default)
///
/// For `ReduceDeficiency` intents, `deficiency_kind_ref` is populated so the
/// yield satisfies spec §2.2 (must reference void OR deficiency).
fn yield_for_intent(
    intent: &SourceIntent,
    evidence: &EvidenceBundle,
    policy_id: Digest,
) -> StructuralYield {
    let (yield_kind, deficiency_kind_ref) = match &intent.kind {
        crate::frontier::SourceIntentKind::ReduceDeficiency { deficiency_kind } => (
            StructuralYieldKind::DeficiencyFill,
            Some(format!("{:?}", deficiency_kind)),
        ),
        crate::frontier::SourceIntentKind::SuggestPattern { .. } => (
            StructuralYieldKind::MotifProposal,
            None,
        ),
        _ => (StructuralYieldKind::DeficiencyFill, None),
    };
    StructuralYield::new(
        yield_kind,
        intent.target_void_id,
        deficiency_kind_ref,
        intent.taint.clone(),
        intent.authority.clone(),
        evidence.bundle_id,
        policy_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{Digest, PolicyProfile};
    use kosmo_workbench::workspace::{WorkspaceEntry, WorkspaceEntryKind, WorkspaceIndex};

    fn make_index(entries: Vec<WorkspaceEntry>) -> WorkspaceIndex {
        let count = entries.len() as u64;
        let mut sorted = entries;
        sorted.sort_by(|a, b| a.path.cmp(&b.path));
        WorkspaceIndex {
            index_id: Digest::of_bytes(b"run-test-index"),
            root: "test".into(),
            entries: sorted,
            policy_id: Digest::ZERO,
            entry_count: count,
        }
    }

    fn src(path: &str) -> WorkspaceEntry {
        WorkspaceEntry {
            path: path.into(),
            digest: Digest::of_bytes(path.as_bytes()),
            size_bytes: 100,
            kind: WorkspaceEntryKind::SourceFile,
        }
    }

    #[test]
    fn passive_run_empty_workspace_produces_empty_result() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(vec![]);
        let result = passive_run(&index, &policy);
        assert_eq!(result.total_yields(), 0);
        assert_eq!(result.host_cube.void_count(), 0);
        assert!(!result.host_cube.has_deficiencies());
        assert!(result.is_clean());
        assert_ne!(result.run_id, Digest::ZERO);
    }

    #[test]
    fn passive_run_source_files_produce_void_yields() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(vec![
            src("src/alpha.rs"),
            src("src/beta.rs"),
        ]);
        let result = passive_run(&index, &policy);
        // Each source file without a test companion generates at least one void
        assert!(result.host_cube.void_count() > 0, "source files must produce voids");
        assert!(result.total_yields() > 0, "voids must produce yields");
        // All yields are Synthetic/Agent → TaintGate/AuthorityGate warn → EvidenceOnly
        assert_eq!(result.rejected_count, 0, "synthetic yields must not be rejected in ReportOnly");
        assert_eq!(result.accepted_count, 0, "synthetic yields must not be accepted in ReportOnly");
    }

    #[test]
    fn passive_run_is_deterministic() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(vec![src("src/lib.rs")]);
        let r1 = passive_run(&index, &policy);
        let r2 = passive_run(&index, &policy);
        assert_eq!(r1.run_id, r2.run_id, "passive_run must be deterministic");
    }

    #[test]
    fn passive_run_summary_is_non_empty() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(vec![src("src/main.rs")]);
        let result = passive_run(&index, &policy);
        let summary = result.summary();
        assert!(!summary.is_empty());
        assert!(summary.contains("HYPHAE v0.3"));
    }

    #[test]
    fn passive_run_deficiency_vector_reflects_voids() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(vec![
            src("src/foo.rs"),
            src("src/bar.rs"),
            src("src/baz.rs"),
        ]);
        let result = passive_run(&index, &policy);
        assert!(result.host_cube.has_deficiencies());
        let dv = &result.host_cube.deficiency_vector;
        assert!(!dv.entries.is_empty(), "deficiency vector must have entries for untested sources");
    }

    #[test]
    fn yield_for_unverified_agent_intent_produces_evidence_only_decision() {
        use kosmo_core::{AuthorityLabel, TaintLabel};
        use crate::frontier::{SourceIntent, SourceIntentKind};
        use crate::gates::GateCascade;
        use crate::assimilation::AssimilationDecision;

        let policy = PolicyProfile::default_report_only();
        let void_id = Digest::of_bytes(b"void-unverified");
        let intent = SourceIntent::new(
            SourceIntentKind::FillVoid { void_id },
            Some(void_id),
            TaintLabel::Unverified,
            AuthorityLabel::Agent { name: "hyphae-v0.3".into() },
        );
        let evidence = synthetic_evidence_for_intent(&intent, policy.id);
        let yield_ = yield_for_intent(&intent, &evidence, policy.id);
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &evidence);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &evidence, policy.id);

        assert!(
            matches!(decision.outcome, crate::assimilation::AssimilationOutcome::EvidenceOnly { .. }),
            "Unverified/Agent intent must produce EvidenceOnly in ReportOnly mode, got {:?}",
            decision.outcome,
        );
        assert_eq!(decision.taint, TaintLabel::Unverified);
    }

    #[test]
    fn yield_for_clean_foundry_intent_produces_accepted_decision() {
        use kosmo_core::{AuthorityLabel, PolicyProfile, TaintLabel};
        use crate::frontier::{SourceIntent, SourceIntentKind};
        use crate::gates::GateCascade;
        use crate::assimilation::AssimilationDecision;

        let policy = PolicyProfile::operator_approved();
        let void_id = Digest::of_bytes(b"void-clean");
        let intent = SourceIntent::new(
            SourceIntentKind::FillVoid { void_id },
            Some(void_id),
            TaintLabel::Clean,
            AuthorityLabel::Foundry,
        );
        let evidence = synthetic_evidence_for_intent(&intent, policy.id);
        let yield_ = yield_for_intent(&intent, &evidence, policy.id);
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &evidence);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &evidence, policy.id);

        assert!(
            decision.outcome.is_accepted(),
            "Clean/Foundry intent must produce Accepted under operator-approved policy, got {:?}",
            decision.outcome,
        );
        assert_eq!(decision.taint, TaintLabel::Clean);
    }

    #[test]
    fn yield_for_reduce_deficiency_intent_carries_deficiency_kind_ref() {
        use kosmo_core::{AuthorityLabel, TaintLabel};
        use crate::deficiency::DeficiencyKind;
        use crate::frontier::{SourceIntent, SourceIntentKind};

        let policy = PolicyProfile::default_report_only();
        let intent = SourceIntent::new(
            SourceIntentKind::ReduceDeficiency { deficiency_kind: DeficiencyKind::TestCoverage },
            None,
            TaintLabel::Unverified,
            AuthorityLabel::Agent { name: "hyphae-v0.3".into() },
        );
        let evidence = synthetic_evidence_for_intent(&intent, policy.id);
        let yield_ = yield_for_intent(&intent, &evidence, policy.id);

        // A ReduceDeficiency intent must produce a yield with deficiency_kind_ref set (spec §2.2).
        assert_eq!(
            yield_.deficiency_kind_ref.as_deref(),
            Some("TestCoverage"),
            "ReduceDeficiency intent must populate deficiency_kind_ref in the yield",
        );
        assert!(
            yield_.host_void_id.is_none(),
            "ReduceDeficiency intent has no specific void to target",
        );
    }

    #[test]
    fn passive_run_includes_reduce_deficiency_intents() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(vec![src("src/alpha.rs"), src("src/beta.rs")]);
        let result = passive_run(&index, &policy);

        // Frontier must contain both FillVoid and ReduceDeficiency intents.
        let fill_count = result.frontier.intents.iter()
            .filter(|i| matches!(&i.kind, crate::frontier::SourceIntentKind::FillVoid { .. }))
            .count();
        let reduce_count = result.frontier.intents.iter()
            .filter(|i| matches!(&i.kind, crate::frontier::SourceIntentKind::ReduceDeficiency { .. }))
            .count();

        assert!(fill_count > 0, "source files must produce FillVoid intents");
        assert!(reduce_count > 0, "deficiency entries must produce ReduceDeficiency intents");
        assert_eq!(
            result.total_yields(),
            fill_count + reduce_count,
            "every intent must produce exactly one decision",
        );
    }

    #[test]
    fn passive_run_augmented_adds_suggest_pattern_intents() {
        use kosmo_core::TaintLabel;
        use crate::frontier::{SourceIntent, SourceIntentKind};

        let policy = PolicyProfile::default_report_only();
        let index = make_index(vec![src("src/lib.rs")]);

        let extra = vec![
            SourceIntent::new(
                SourceIntentKind::SuggestPattern { pattern_name: "motif:MissingTestFiber".into() },
                None,
                TaintLabel::Unverified,
                kosmo_core::AuthorityLabel::Agent { name: "hyphae-v0.3".into() },
            ),
        ];
        let base = passive_run(&index, &policy);
        let augmented = passive_run_augmented(&index, &policy, extra);

        assert_eq!(
            augmented.total_yields(),
            base.total_yields() + 1,
            "one extra SuggestPattern intent must produce one extra decision",
        );
        let suggest_count = augmented.frontier.intents.iter()
            .filter(|i| matches!(&i.kind, SourceIntentKind::SuggestPattern { .. }))
            .count();
        assert_eq!(suggest_count, 1, "frontier must contain the injected SuggestPattern intent");
        // run_id must differ since frontier_id differs (INVARIANT-007).
        assert_ne!(base.run_id, augmented.run_id, "augmented run must have different run_id");
    }

    #[test]
    fn yield_for_suggest_pattern_intent_uses_motif_proposal_kind() {
        use kosmo_core::{AuthorityLabel, TaintLabel};
        use crate::frontier::{SourceIntent, SourceIntentKind};

        let policy = PolicyProfile::default_report_only();
        let intent = SourceIntent::new(
            SourceIntentKind::SuggestPattern { pattern_name: "motif:MissingTestFiber".into() },
            None,
            TaintLabel::Unverified,
            AuthorityLabel::Agent { name: "hyphae-v0.3".into() },
        );
        let evidence = synthetic_evidence_for_intent(&intent, policy.id);
        let yield_ = yield_for_intent(&intent, &evidence, policy.id);

        assert_eq!(
            yield_.kind,
            StructuralYieldKind::MotifProposal,
            "SuggestPattern intent must produce MotifProposal yield",
        );
        assert!(yield_.host_void_id.is_none(), "MotifProposal yield has no specific void");
        assert!(yield_.deficiency_kind_ref.is_none(), "MotifProposal yield has no deficiency ref");
    }
}

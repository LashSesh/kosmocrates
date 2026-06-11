use crate::workspace::WorkspaceIndex;
use kosmo_core::{
    Digest, EvidenceBundle, FoundryCheckResult, FoundryOutcome, ImplementationMode, RunDescriptor,
};
use serde::{Deserialize, Serialize};

/// Serialize-only content struct for content-addressing a RunReport.
#[derive(Serialize)]
struct ReportContent<'a> {
    run_id: &'a Digest,
    mode: &'a ImplementationMode,
    workspace_index_id: &'a Option<Digest>,
    foundry_result_ids: Vec<Digest>,
    evidence_bundle_id: &'a Digest,
    notes: &'a [String],
}

/// An immutable, content-addressed dry-run / report-only output.
///
/// Produced at the end of a Workbench run. Carries:
/// - identity of the run and policy,
/// - workspace summary,
/// - Foundry check results,
/// - the evidence bundle that chains all artifacts,
/// - human-readable notes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunReport {
    pub report_id: Digest,
    pub run_id: Digest,
    pub policy_id: Digest,
    pub mode: ImplementationMode,
    pub workspace_entry_count: u64,
    pub workspace_index_id: Option<Digest>,
    pub foundry_results: Vec<FoundryCheckResult>,
    pub evidence_bundle_id: Digest,
    pub notes: Vec<String>,
}

impl RunReport {
    pub fn new(
        run: &RunDescriptor,
        workspace: Option<&WorkspaceIndex>,
        foundry_results: Vec<FoundryCheckResult>,
        evidence_bundle: &EvidenceBundle,
        mode: ImplementationMode,
        mut notes: Vec<String>,
    ) -> Self {
        notes.push(format!("Mode: {:?}. No host files were modified.", mode));

        let workspace_index_id = workspace.map(|w| w.index_id);
        let workspace_entry_count = workspace.map(|w| w.entry_count).unwrap_or(0);

        let foundry_result_ids: Vec<Digest> = foundry_results.iter().map(|r| r.check_id).collect();

        let report_id = Digest::of(&ReportContent {
            run_id: &run.run_id,
            mode: &mode,
            workspace_index_id: &workspace_index_id,
            foundry_result_ids,
            evidence_bundle_id: &evidence_bundle.bundle_id,
            notes: &notes,
        });

        Self {
            report_id,
            run_id: run.run_id,
            policy_id: run.policy_id,
            mode,
            workspace_entry_count,
            workspace_index_id,
            foundry_results,
            evidence_bundle_id: evidence_bundle.bundle_id,
            notes,
        }
    }

    /// Human-readable operator-facing summary.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str("=== Kosmocrates Workbench Run Report ===\n");
        out.push_str(&format!("Report ID    : {}\n", self.report_id));
        out.push_str(&format!("Run ID       : {}\n", self.run_id));
        out.push_str(&format!("Policy ID    : {}\n", self.policy_id));
        out.push_str(&format!("Mode         : {:?}\n", self.mode));
        out.push_str(&format!(
            "Workspace    : {} file(s)",
            self.workspace_entry_count
        ));
        if let Some(wid) = &self.workspace_index_id {
            out.push_str(&format!(" [{}]\n", wid));
        } else {
            out.push_str(" [no scan]\n");
        }
        out.push_str(&format!("Evidence     : {}\n", self.evidence_bundle_id));

        if !self.foundry_results.is_empty() {
            out.push_str("\nFoundry Checks:\n");
            for r in &self.foundry_results {
                let status = match &r.outcome {
                    FoundryOutcome::Passed => "PASS".to_string(),
                    FoundryOutcome::Failed { exit_code, .. } => format!("FAIL({})", exit_code),
                    FoundryOutcome::Skipped { reason } => format!("SKIP: {}", reason),
                    FoundryOutcome::Unavailable { reason } => format!("UNAVAIL: {}", reason),
                };
                out.push_str(&format!("  [{:?}] {}\n", r.check_kind, status));
            }
        }

        if !self.notes.is_empty() {
            out.push_str("\nNotes:\n");
            for note in &self.notes {
                out.push_str(&format!("  - {}\n", note));
            }
        }

        out
    }

    pub fn passed_foundry_count(&self) -> usize {
        self.foundry_results
            .iter()
            .filter(|r| r.outcome.is_passed())
            .count()
    }

    pub fn skipped_foundry_count(&self) -> usize {
        self.foundry_results
            .iter()
            .filter(|r| matches!(r.outcome, FoundryOutcome::Skipped { .. }))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundry::FoundryRunner;
    use crate::workspace::WorkspaceIndex;
    use kosmo_core::{Digest, EvidenceBundle, ImplementationMode, PolicyProfile, RunDescriptor};

    fn make_run(policy: &PolicyProfile) -> RunDescriptor {
        RunDescriptor::new(policy.id, "/workspace/test")
    }

    #[test]
    fn run_report_has_nonzero_id() {
        let policy = PolicyProfile::default_report_only();
        let run = make_run(&policy);
        let bundle = EvidenceBundle::empty(policy.id);
        let report = RunReport::new(
            &run,
            None,
            vec![],
            &bundle,
            ImplementationMode::ReportOnly,
            vec![],
        );
        assert_ne!(report.report_id, Digest::ZERO);
    }

    #[test]
    fn run_report_deterministic() {
        let policy = PolicyProfile::default_report_only();
        let run = make_run(&policy);
        let bundle = EvidenceBundle::empty(policy.id);
        let r1 = RunReport::new(
            &run,
            None,
            vec![],
            &bundle,
            ImplementationMode::ReportOnly,
            vec![],
        );
        let r2 = RunReport::new(
            &run,
            None,
            vec![],
            &bundle,
            ImplementationMode::ReportOnly,
            vec![],
        );
        assert_eq!(r1.report_id, r2.report_id);
    }

    #[test]
    fn run_report_with_workspace_and_foundry() {
        let policy = PolicyProfile::default_report_only();
        let run = make_run(&policy);
        let pid = Digest::of_bytes(b"pid");

        // Build a tiny workspace index
        let idx = WorkspaceIndex::from_entries("/workspace/test".into(), vec![], pid);

        // Run foundry in report-only mode (will all be Skipped)
        let foundry_output = FoundryRunner::standard_checks(policy.clone()).run_all(idx.index_id);

        let report = RunReport::new(
            &run,
            Some(&idx),
            foundry_output.results,
            &foundry_output.evidence_bundle,
            ImplementationMode::ReportOnly,
            vec![],
        );

        assert_eq!(report.workspace_entry_count, 0);
        assert_eq!(report.skipped_foundry_count(), 2);
        assert_ne!(report.report_id, Digest::ZERO);

        // to_text should mention ReportOnly
        let text = report.to_text();
        assert!(text.contains("ReportOnly"));
        assert!(text.contains("SKIP"));
    }

    #[test]
    fn cross_013_report_only_produces_useful_diagnostics_without_writing() {
        // CROSS-013: Report-only mode produces useful diagnostics without writing host files.
        let policy = PolicyProfile::default_report_only();
        let run = make_run(&policy);
        let bundle = EvidenceBundle::empty(policy.id);
        let report = RunReport::new(
            &run,
            None,
            vec![],
            &bundle,
            ImplementationMode::ReportOnly,
            vec!["Phase 2 test run".into()],
        );
        let text = report.to_text();
        assert!(!text.is_empty());
        assert!(text.contains("Run Report"));
        assert!(text.contains("Phase 2 test run"));
        // host_write was never attempted — policy enforces it
        assert_eq!(
            policy.check_host_write().unwrap_err().to_string(),
            "host write denied by policy (ReportOnly or allow_host_write=false)"
        );
    }
}

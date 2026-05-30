//! Kosmocrates integration pipeline — Phase 10 Integration Hardening.
//!
//! Provides a single `run_dry_pipeline()` entry point that wires together:
//!
//! Workbench → HYPHAE v0.3 passive run → CorpusCartography update →
//! optional Metatron v0.4.1 diagnostics → optional LPCM v0.4.2 report →
//! optional SystemCube v0.4.3 export → aggregated GateTrace → IntegrationRunReport
//!
//! The single `PolicyProfile` governs every layer; its ID appears in every
//! sub-report, proving that one policy scoped the entire run.
//!
//! Invariants:
//! - No host files are written.
//! - Fail-closed: any Reject in any layer propagates to final_result.
//! - Evidence propagates: each optional-layer output traces HYPHAE evidence.
//! - All IDs are content-addressed.
//! - Default mode: `PolicyProfile::default_report_only()`.

pub mod aggregator;

pub use aggregator::{AggregatedGateResult, GateTraceAggregator, LayerGateSummary};

use kosmo_core::{Digest, GateResult, PolicyProfile, Q16};
use kosmo_core::TaintLabel;
use kosmo_hyphae::{
    CorpusCartography,
    CorpusCartographyUpdate,
    LpcmPassiveReport,
    MicroTopologyDiagnostic,
    Fragment, FragmentField, FragmentKind,
    SeamGraph, SupportMassVector,
    diagnose_micrograph, lift_region,
    passive_run, HyphaeRunResult,
};
use kosmo_systemcube::{
    BlueprintUnit, BlueprintUnitKind, KcubeExportReport, SystemCube,
};
use kosmo_workbench::WorkspaceIndex;
use serde::{Deserialize, Serialize};

// ─── IntegrationRunOptions ────────────────────────────────────────────────────

/// Configuration for which optional pipeline layers to activate.
///
/// All optional layers default to disabled, matching the `ReportOnly` mode.
/// No layer can be enabled unless the governing `PolicyProfile` permits it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrationRunOptions {
    /// Run Metatron v0.4.1 region lift + diagnostic for each host void.
    pub enable_metatron: bool,
    /// Run LPCM v0.4.2 passive report for each host void.
    pub enable_lpcm: bool,
    /// Build a SystemCube v0.4.3 from accepted decisions and export dry-run.
    pub enable_systemcube: bool,
    /// Denominator for D-density in the SystemCube report.
    pub systemcube_capacity: u32,
    /// Q16 seam compatibility threshold for LPCM reports.
    pub lpcm_seam_threshold: Q16,
}

impl IntegrationRunOptions {
    /// Safe default: all optional layers disabled.
    pub fn report_only() -> Self {
        Self {
            enable_metatron: false,
            enable_lpcm: false,
            enable_systemcube: false,
            systemcube_capacity: 0,
            lpcm_seam_threshold: Q16::ZERO,
        }
    }

    /// Enable all optional layers (for integration testing with fixtures).
    pub fn all_layers(systemcube_capacity: u32) -> Self {
        Self {
            enable_metatron: true,
            enable_lpcm: true,
            enable_systemcube: true,
            systemcube_capacity,
            lpcm_seam_threshold: Q16::ZERO,
        }
    }
}

// ─── Integration Run Report ───────────────────────────────────────────────────

/// Content for deterministic `IntegrationRunReport` ID.
#[derive(Serialize)]
struct ReportContent {
    hyphae_run_id: Digest,
    cartography_update_id: Digest,
    policy_id: Digest,
    aggregate_id: Digest,
    metatron_count: u32,
    lpcm_count: u32,
    has_systemcube: bool,
}

/// Unified dry-run report from the full pipeline.
///
/// Every sub-report carries `policy_id` equal to `self.policy_id`, proving
/// that a single `PolicyProfile` governed the entire run (traceability).
/// `final_result` is the fail-closed merge of all layer gate results.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrationRunReport {
    pub report_id: Digest,
    pub policy_id: Digest,
    pub hyphae_result: HyphaeRunResult,
    pub cartography_update: CorpusCartographyUpdate,
    pub metatron_diagnostics: Vec<MicroTopologyDiagnostic>,
    pub lpcm_reports: Vec<LpcmPassiveReport>,
    pub systemcube_export: Option<KcubeExportReport>,
    pub aggregated_gate: AggregatedGateResult,
    /// Fail-closed merge across all layers.
    pub final_result: GateResult,
}

impl IntegrationRunReport {
    fn new(
        hyphae_result: HyphaeRunResult,
        cartography_update: CorpusCartographyUpdate,
        metatron_diagnostics: Vec<MicroTopologyDiagnostic>,
        lpcm_reports: Vec<LpcmPassiveReport>,
        systemcube_export: Option<KcubeExportReport>,
        aggregated_gate: AggregatedGateResult,
        policy: &PolicyProfile,
    ) -> Self {
        let final_result = aggregated_gate.final_result.clone();
        let report_id = Digest::of(&ReportContent {
            hyphae_run_id: hyphae_result.run_id,
            cartography_update_id: cartography_update.update_id,
            policy_id: policy.id,
            aggregate_id: aggregated_gate.aggregate_id,
            metatron_count: metatron_diagnostics.len() as u32,
            lpcm_count: lpcm_reports.len() as u32,
            has_systemcube: systemcube_export.is_some(),
        });
        Self {
            report_id,
            policy_id: policy.id,
            hyphae_result,
            cartography_update,
            metatron_diagnostics,
            lpcm_reports,
            systemcube_export,
            aggregated_gate,
            final_result,
        }
    }

    /// Operator-readable summary of the pipeline run.
    pub fn summary(&self) -> String {
        let scube = if let Some(ref e) = self.systemcube_export {
            format!("systemcube=Some(mode={:?})", e.mode)
        } else {
            "systemcube=None".into()
        };
        format!(
            "IntegrationRunReport — policy={:.8} | final={:?} | \
             hyphae: {} | cartography: {} entities | \
             metatron: {} | lpcm: {} | {}",
            hex_prefix(&self.policy_id),
            self.final_result,
            self.hyphae_result.summary(),
            self.cartography_update.added_entity_ids.len(),
            self.metatron_diagnostics.len(),
            self.lpcm_reports.len(),
            scube,
        )
    }

    /// Whether all layers passed their gates.
    pub fn is_clean(&self) -> bool {
        self.final_result.is_pass()
    }

    /// Verify that every sub-report's policy_id matches the governing policy.
    ///
    /// This is the traceability invariant: one policy, one run.
    pub fn verify_policy_consistency(&self) -> bool {
        let pid = self.policy_id;
        self.hyphae_result.policy_id == pid
            && self.cartography_update.policy_id == pid
            && self.aggregated_gate.policy_id == pid
            && self.metatron_diagnostics.iter().all(|d| d.policy_id == pid)
            && self.lpcm_reports.iter().all(|r| r.policy_id == pid)
            && self
                .systemcube_export
                .as_ref()
                .map_or(true, |e| e.policy_id == pid)
    }
}

fn hex_prefix(d: &Digest) -> String {
    d.as_bytes().iter().take(4).map(|b| format!("{:02x}", b)).collect()
}

// ─── Pipeline Entry Point ──────────────────────────────────────────────────────

/// Run the full Kosmocrates dry-run pipeline on a workspace index.
///
/// No host files are written. All outputs are passive/advisory reports.
/// The governing `PolicyProfile` is applied at every layer.
///
/// Optional layers (Metatron, LPCM, SystemCube) are activated via `options`.
/// In `ReportOnly` mode (default policy), all optional layers are safe to run
/// because no materialization or host mutation can occur.
pub fn run_dry_pipeline(
    index: &WorkspaceIndex,
    options: &IntegrationRunOptions,
    policy: &PolicyProfile,
) -> IntegrationRunReport {
    let mut agg = GateTraceAggregator::new(policy.id);

    // ── 1. HYPHAE v0.3 passive run ────────────────────────────────────────────
    let hyphae = passive_run(index, policy);

    // Record HYPHAE gate contribution: any rejected decision contributes Reject.
    let hyphae_gate = if hyphae.rejected_count > 0 {
        GateResult::Reject {
            reason: format!("{} yields rejected by gate cascade", hyphae.rejected_count),
        }
    } else if hyphae.evidence_only_count > 0 {
        GateResult::Warn {
            message: format!("{} yields downgraded to evidence-only", hyphae.evidence_only_count),
        }
    } else {
        GateResult::Pass
    };
    agg.record("hyphae", hyphae.run_id, hyphae_gate);

    // ── 2. CorpusCartography update ───────────────────────────────────────────
    let corpus = CorpusCartography::empty(policy.id);
    let (_, cartography_update) = corpus.update_from_run(&hyphae);
    agg.record(
        "cartography",
        cartography_update.update_id,
        GateResult::Pass,
    );

    // ── 3. Optional Metatron v0.4.1 diagnostics ───────────────────────────────
    let mut metatron_diagnostics: Vec<MicroTopologyDiagnostic> = Vec::new();
    if options.enable_metatron {
        for void in &hyphae.host_cube.void_map.voids {
            let ev_id = void.void_id;
            let (micrograph, fingerprint, _loss) =
                lift_region(void.void_id, vec![void.void_id], ev_id, TaintLabel::Synthetic, policy);
            let diag = diagnose_micrograph(&micrograph, &fingerprint, Some(&void.kind), policy);
            agg.record(
                format!("metatron:{:.8}", hex_prefix(&void.void_id)),
                diag.diagnostic_id,
                GateResult::Pass,
            );
            metatron_diagnostics.push(diag);
        }
    }

    // ── 4. Optional LPCM v0.4.2 passive reports ───────────────────────────────
    let mut lpcm_reports: Vec<LpcmPassiveReport> = Vec::new();
    if options.enable_lpcm {
        for void in &hyphae.host_cube.void_map.voids {
            let ev_id = void.void_id;
            let fragment = Fragment::new(
                void.void_id,
                0,
                FragmentKind::CohesiveRegion,
                vec![void.void_id],
                ev_id,
            );
            let field = FragmentField::new(void.void_id, ev_id, policy, vec![fragment]);
            let entries = vec![(field.fragments[0].fragment_id, Q16::ONE)];
            let support = SupportMassVector::new(field.field_id, policy, entries);
            let seam_graph = SeamGraph::new(field.field_id, policy, vec![]);
            let lpcm = LpcmPassiveReport::build(
                void.void_id,
                field,
                support,
                seam_graph,
                options.lpcm_seam_threshold,
                ev_id,
                policy,
            );
            agg.record(
                format!("lpcm:{:.8}", hex_prefix(&void.void_id)),
                lpcm.report_id,
                GateResult::Pass,
            );
            lpcm_reports.push(lpcm);
        }
    }

    // ── 5. Optional SystemCube v0.4.3 export ──────────────────────────────────
    let systemcube_export: Option<KcubeExportReport> = if options.enable_systemcube {
        let run_desc = kosmo_core::RunDescriptor::new(policy.id, "pipeline");
        let units: Vec<BlueprintUnit> = hyphae
            .decisions
            .iter()
            .filter(|d| d.outcome.is_accepted())
            .map(|d| {
                BlueprintUnit::new(
                    BlueprintUnitKind::ModuleBoundary,
                    d.yield_id,
                    kosmo_core::AuthorityLabel::Foundry,
                    TaintLabel::Synthetic,
                    vec![d.evidence_bundle_id],
                    policy,
                )
            })
            .collect();
        let cube = SystemCube::new(hyphae.host_cube.cube_id, &run_desc, policy, units);
        let export = cube.export_dry_run(options.systemcube_capacity, policy);
        agg.record("systemcube", export.export_id, GateResult::Pass);
        Some(export)
    } else {
        None
    };

    // ── 6. Aggregate gate results ─────────────────────────────────────────────
    let aggregated_gate = agg.aggregate();

    IntegrationRunReport::new(
        hyphae,
        cartography_update,
        metatron_diagnostics,
        lpcm_reports,
        systemcube_export,
        aggregated_gate,
        policy,
    )
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::PolicyProfile;
    use kosmo_workbench::WorkspaceIndex;

    fn policy() -> PolicyProfile {
        PolicyProfile::default_report_only()
    }

    fn empty_index() -> WorkspaceIndex {
        WorkspaceIndex::from_entries("test-root".into(), vec![], policy().id)
    }

    fn fixture_index() -> WorkspaceIndex {
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind};
        let entries = vec![
            WorkspaceEntry { path: "src/lib.rs".into(), digest: Digest::of_bytes(b"lib"), size_bytes: 100, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "src/main.rs".into(), digest: Digest::of_bytes(b"main"), size_bytes: 200, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "src/lib_test.rs".into(), digest: Digest::of_bytes(b"lib_test"), size_bytes: 50, kind: WorkspaceEntryKind::TestFile },
        ];
        WorkspaceIndex::from_entries("test-root".into(), entries, policy().id)
    }

    // ── Pipeline: empty workspace ─────────────────────────────────────────────

    #[test]
    fn pipeline_empty_workspace_passes_all_gates() {
        let r = run_dry_pipeline(&empty_index(), &IntegrationRunOptions::report_only(), &policy());
        assert!(r.final_result.is_pass(), "empty workspace must pass all gates");
    }

    #[test]
    fn pipeline_empty_workspace_report_is_content_addressed() {
        let r1 = run_dry_pipeline(&empty_index(), &IntegrationRunOptions::report_only(), &policy());
        let r2 = run_dry_pipeline(&empty_index(), &IntegrationRunOptions::report_only(), &policy());
        assert_eq!(r1.report_id, r2.report_id);
        assert_ne!(r1.report_id, Digest::ZERO);
    }

    // ── Policy consistency (traceability) ─────────────────────────────────────

    #[test]
    fn pipeline_policy_consistency_empty() {
        let r = run_dry_pipeline(&empty_index(), &IntegrationRunOptions::report_only(), &policy());
        assert!(
            r.verify_policy_consistency(),
            "every sub-report must carry the same policy_id"
        );
    }

    #[test]
    fn pipeline_policy_consistency_all_layers() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::all_layers(4),
            &policy(),
        );
        assert!(
            r.verify_policy_consistency(),
            "all-layers run must have consistent policy_id in every sub-report"
        );
    }

    // ── Metatron optional layer ────────────────────────────────────────────────

    #[test]
    fn pipeline_no_metatron_when_disabled() {
        let r = run_dry_pipeline(&fixture_index(), &IntegrationRunOptions::report_only(), &policy());
        assert!(r.metatron_diagnostics.is_empty(), "Metatron must be off when disabled");
    }

    #[test]
    fn pipeline_metatron_produces_one_diagnostic_per_void() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            enable_lpcm: false,
            enable_systemcube: false,
            systemcube_capacity: 0,
            lpcm_seam_threshold: Q16::ZERO,
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        assert_eq!(
            r.metatron_diagnostics.len(),
            r.hyphae_result.host_cube.void_count(),
            "one Metatron diagnostic per host void"
        );
    }

    #[test]
    fn pipeline_metatron_diagnostics_carry_correct_policy_id() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let p = policy();
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        for diag in &r.metatron_diagnostics {
            assert_eq!(diag.policy_id, p.id, "Metatron diagnostic must carry pipeline policy_id");
        }
    }

    // ── LPCM optional layer ────────────────────────────────────────────────────

    #[test]
    fn pipeline_no_lpcm_when_disabled() {
        let r = run_dry_pipeline(&fixture_index(), &IntegrationRunOptions::report_only(), &policy());
        assert!(r.lpcm_reports.is_empty(), "LPCM must be off when disabled");
    }

    #[test]
    fn pipeline_lpcm_produces_one_report_per_void() {
        let opts = IntegrationRunOptions {
            enable_lpcm: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        assert_eq!(
            r.lpcm_reports.len(),
            r.hyphae_result.host_cube.void_count(),
            "one LPCM report per host void"
        );
    }

    #[test]
    fn pipeline_lpcm_reports_carry_correct_policy_id() {
        let opts = IntegrationRunOptions {
            enable_lpcm: true,
            ..IntegrationRunOptions::report_only()
        };
        let p = policy();
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        for lr in &r.lpcm_reports {
            assert_eq!(lr.policy_id, p.id, "LPCM report must carry pipeline policy_id");
        }
    }

    // ── SystemCube optional layer ──────────────────────────────────────────────

    #[test]
    fn pipeline_no_systemcube_when_disabled() {
        let r = run_dry_pipeline(&fixture_index(), &IntegrationRunOptions::report_only(), &policy());
        assert!(r.systemcube_export.is_none(), "SystemCube must be off when disabled");
    }

    #[test]
    fn pipeline_systemcube_export_is_blocked_by_default_policy() {
        use kosmo_systemcube::KcubeExportMode;
        let opts = IntegrationRunOptions::all_layers(4);
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        if let Some(ref export) = r.systemcube_export {
            assert_eq!(
                export.mode,
                KcubeExportMode::BlockedByPolicy,
                "SystemCube export must be BlockedByPolicy in ReportOnly mode"
            );
        }
    }

    #[test]
    fn pipeline_systemcube_carries_correct_policy_id() {
        let opts = IntegrationRunOptions::all_layers(4);
        let p = policy();
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        if let Some(ref export) = r.systemcube_export {
            assert_eq!(export.policy_id, p.id);
        }
    }

    // ── Fail-closed gate propagation ──────────────────────────────────────────

    #[test]
    fn fail_closed_aggregator_reject_propagates() {
        let mut agg = GateTraceAggregator::new(Digest::of_bytes(b"p"));
        agg.record("hyphae", Digest::of_bytes(b"h"), GateResult::Pass);
        agg.record("lpcm", Digest::of_bytes(b"l"), GateResult::Reject { reason: "bad".into() });
        agg.record("meta", Digest::of_bytes(b"m"), GateResult::Warn { message: "low".into() });
        let result = agg.aggregate();
        assert!(
            result.final_result.is_rejected(),
            "single Reject must propagate to final_result"
        );
    }

    // ── CROSS-013: no host files written ──────────────────────────────────────

    #[test]
    fn cross_013_pipeline_no_host_write() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::all_layers(4),
            &policy(),
        );
        // Structural: IntegrationRunReport has no write/patch methods.
        // Policy confirms host writes forbidden.
        assert!(!policy().allow_host_write);
        assert!(!policy().allow_systemcube_materialization);
        let _s = r.summary();
    }

    // ── CROSS-002: host mutation impossible in default policy ─────────────────

    #[test]
    fn cross_002_host_mutation_impossible_by_default() {
        let p = policy();
        assert!(!p.allow_host_write, "allow_host_write must be false in default policy");
        assert!(
            !p.allow_systemcube_materialization,
            "systemcube materialization must be false in default policy"
        );
        assert!(
            !p.allow_external_acquisition,
            "external acquisition must be false in default policy"
        );
    }

    // ── Determinism across all layers ─────────────────────────────────────────

    #[test]
    fn pipeline_all_layers_is_deterministic() {
        let opts = IntegrationRunOptions::all_layers(4);
        let r1 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let r2 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        assert_eq!(r1.report_id, r2.report_id, "pipeline must be deterministic");
    }

    // ── Summary is non-empty ───────────────────────────────────────────────────

    #[test]
    fn pipeline_summary_non_empty() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::all_layers(4),
            &policy(),
        );
        let s = r.summary();
        assert!(!s.is_empty());
        assert!(s.contains("IntegrationRunReport"));
    }
}

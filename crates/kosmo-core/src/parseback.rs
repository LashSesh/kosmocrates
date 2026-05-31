use crate::digest::Digest;
use crate::evidence::EvidenceRef;
use serde::{Deserialize, Serialize};

/// Scope of the topology rescan performed after a materialization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParseBackScanScope {
    /// Rescan only files directly modified by the materialization.
    AffectedFilesOnly,
    /// Rescan all crates whose files were touched.
    AffectedCratesOnly,
    /// Full workspace rescan — highest fidelity, highest cost.
    FullWorkspace,
    Custom(String),
}

/// Classifies the kind of topology change detected by a parse-back rescan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopologyChangeKind {
    NodeAdded,
    NodeRemoved,
    NodeModified,
    EdgeAdded,
    EdgeRemoved,
    EdgeModified,
    ClusterBoundaryChanged,
    VoidMapChanged,
    StructuralCrystalAffected,
    Custom(String),
}

/// Severity of a detected topology change.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ParseBackSeverity {
    Info,
    Warning,
    Critical,
}

/// A single content-addressed topology change detected during parse-back.
///
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseBackTopologyDelta {
    pub id: Digest,
    pub change_kind: TopologyChangeKind,
    /// Digest identifying the node or edge involved, if applicable.
    pub node_id: Option<Digest>,
    pub description: String,
    pub severity: ParseBackSeverity,
}

#[derive(Serialize)]
struct DeltaContent<'a> {
    change_kind: &'a TopologyChangeKind,
    node_id: &'a Option<Digest>,
    description: &'a String,
    severity: &'a ParseBackSeverity,
}

impl ParseBackTopologyDelta {
    pub fn new(
        change_kind: TopologyChangeKind,
        node_id: Option<Digest>,
        description: impl Into<String>,
        severity: ParseBackSeverity,
    ) -> Self {
        let mut d = Self {
            id: Digest::ZERO,
            change_kind,
            node_id,
            description: description.into(),
            severity,
        };
        d.id = d.compute_id();
        d
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&DeltaContent {
            change_kind: &self.change_kind,
            node_id: &self.node_id,
            description: &self.description,
            severity: &self.severity,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    pub fn is_critical(&self) -> bool {
        self.severity == ParseBackSeverity::Critical
    }
}

/// Overall outcome of a parse-back topology rescan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParseBackOutcome {
    /// Rescan was skipped because `PolicyProfile.mode == ReportOnly`.
    SkippedByReportOnly,
    /// Rescan passed — topology is consistent with the materialization.
    Passed,
    /// Rescan failed — topology inconsistency or structural violation detected.
    Failed,
    /// No topology changes detected; materialization was topology-neutral.
    TopologyUnchanged,
    /// Result cannot be determined (e.g., parse error during rescan).
    Inconclusive,
}

impl ParseBackOutcome {
    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed | Self::TopologyUnchanged)
    }

    pub fn is_skipped_report_only(&self) -> bool {
        matches!(self, Self::SkippedByReportOnly)
    }

    pub fn is_failure_class(&self) -> bool {
        matches!(self, Self::Failed | Self::Inconclusive)
    }
}

/// Specifies the scope and baseline for a parse-back rescan.
///
/// Created before materialization; references the pre-materialization topology
/// digest so the post-materialization rescan can compute a `ParseBackTopologyDelta`.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseBackPlan {
    pub id: Digest,
    pub policy_id: Digest,
    pub materialization_plan_id: Digest,
    pub scan_scope: ParseBackScanScope,
    /// Content digest of the topology graph immediately before materialization.
    pub baseline_topology_id: Digest,
    pub evidence_refs: Vec<EvidenceRef>,
}

#[derive(Serialize)]
struct PlanContent<'a> {
    policy_id: &'a Digest,
    materialization_plan_id: &'a Digest,
    scan_scope: &'a ParseBackScanScope,
    baseline_topology_id: &'a Digest,
    evidence_digests: Vec<&'a Digest>,
}

impl ParseBackPlan {
    pub fn new(
        policy_id: Digest,
        materialization_plan_id: Digest,
        scan_scope: ParseBackScanScope,
        baseline_topology_id: Digest,
    ) -> Self {
        let mut p = Self {
            id: Digest::ZERO,
            policy_id,
            materialization_plan_id,
            scan_scope,
            baseline_topology_id,
            evidence_refs: vec![],
        };
        p.id = p.compute_id();
        p
    }

    pub fn with_evidence(mut self, refs: Vec<EvidenceRef>) -> Self {
        self.evidence_refs = refs;
        self.id = self.compute_id();
        self
    }

    fn compute_id(&self) -> Digest {
        let evidence_digests: Vec<&Digest> = self.evidence_refs.iter().map(|e| &e.digest).collect();
        Digest::of(&PlanContent {
            policy_id: &self.policy_id,
            materialization_plan_id: &self.materialization_plan_id,
            scan_scope: &self.scan_scope,
            baseline_topology_id: &self.baseline_topology_id,
            evidence_digests,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// The complete, content-addressed record of a parse-back rescan.
///
/// `pre_topology_id` matches `ParseBackPlan::baseline_topology_id`.
/// `post_topology_id` is the topology digest after materialization.
/// `deltas` lists every detected change; empty when `TopologyUnchanged`.
/// `evidence_bundle_id` is mandatory — every durable report must be evidence-bound.
/// `elapsed_ms` is `u64` — no floats in any gate/audit path.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseBackReport {
    pub id: Digest,
    pub plan_id: Digest,
    pub outcome: ParseBackOutcome,
    pub deltas: Vec<ParseBackTopologyDelta>,
    pub pre_topology_id: Digest,
    pub post_topology_id: Digest,
    pub diagnostics: Vec<String>,
    pub evidence_bundle_id: Digest,
    pub elapsed_ms: u64,
}

#[derive(Serialize)]
struct ReportContent<'a> {
    plan_id: &'a Digest,
    outcome: &'a ParseBackOutcome,
    delta_ids: Vec<&'a Digest>,
    pre_topology_id: &'a Digest,
    post_topology_id: &'a Digest,
    diagnostics: &'a Vec<String>,
    evidence_bundle_id: &'a Digest,
    elapsed_ms: u64,
}

impl ParseBackReport {
    pub fn skipped_by_report_only(
        plan_id: Digest,
        pre_topology_id: Digest,
        evidence_bundle_id: Digest,
    ) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            plan_id,
            outcome: ParseBackOutcome::SkippedByReportOnly,
            deltas: vec![],
            pre_topology_id,
            post_topology_id: pre_topology_id,
            diagnostics: vec!["Skipped: PolicyProfile.mode == ReportOnly".into()],
            evidence_bundle_id,
            elapsed_ms: 0,
        };
        r.id = r.compute_id();
        r
    }

    pub fn topology_unchanged(
        plan_id: Digest,
        topology_id: Digest,
        evidence_bundle_id: Digest,
        elapsed_ms: u64,
    ) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            plan_id,
            outcome: ParseBackOutcome::TopologyUnchanged,
            deltas: vec![],
            pre_topology_id: topology_id,
            post_topology_id: topology_id,
            diagnostics: vec![],
            evidence_bundle_id,
            elapsed_ms,
        };
        r.id = r.compute_id();
        r
    }

    pub fn new(
        plan_id: Digest,
        outcome: ParseBackOutcome,
        deltas: Vec<ParseBackTopologyDelta>,
        pre_topology_id: Digest,
        post_topology_id: Digest,
        diagnostics: Vec<String>,
        evidence_bundle_id: Digest,
        elapsed_ms: u64,
    ) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            plan_id,
            outcome,
            deltas,
            pre_topology_id,
            post_topology_id,
            diagnostics,
            evidence_bundle_id,
            elapsed_ms,
        };
        r.id = r.compute_id();
        r
    }

    fn compute_id(&self) -> Digest {
        let delta_ids: Vec<&Digest> = self.deltas.iter().map(|d| &d.id).collect();
        Digest::of(&ReportContent {
            plan_id: &self.plan_id,
            outcome: &self.outcome,
            delta_ids,
            pre_topology_id: &self.pre_topology_id,
            post_topology_id: &self.post_topology_id,
            diagnostics: &self.diagnostics,
            evidence_bundle_id: &self.evidence_bundle_id,
            elapsed_ms: self.elapsed_ms,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    /// Returns `true` if any delta has `Critical` severity.
    pub fn has_critical_delta(&self) -> bool {
        self.deltas.iter().any(|d| d.is_critical())
    }

    /// Worst-severity delta in the report; `None` when `deltas` is empty.
    pub fn worst_severity(&self) -> Option<&ParseBackSeverity> {
        self.deltas.iter().map(|d| &d.severity).max()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{EvidenceBundle, EvidenceKind, ReplayStatus};

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn policy_id() -> Digest { d(b"policy") }
    fn mat_plan_id() -> Digest { d(b"mat-plan") }
    fn topo_id() -> Digest { d(b"topo-baseline") }
    fn bundle_id() -> Digest { d(b"bundle") }

    fn basic_plan() -> ParseBackPlan {
        ParseBackPlan::new(
            policy_id(),
            mat_plan_id(),
            ParseBackScanScope::AffectedCratesOnly,
            topo_id(),
        )
    }

    // --- ParseBackTopologyDelta ---

    #[test]
    fn delta_id_deterministic() {
        let d1 = ParseBackTopologyDelta::new(
            TopologyChangeKind::NodeAdded,
            None,
            "new module detected",
            ParseBackSeverity::Info,
        );
        let d2 = ParseBackTopologyDelta::new(
            TopologyChangeKind::NodeAdded,
            None,
            "new module detected",
            ParseBackSeverity::Info,
        );
        assert_eq!(d1.id, d2.id);
    }

    #[test]
    fn delta_verify_id() {
        let delta = ParseBackTopologyDelta::new(
            TopologyChangeKind::EdgeRemoved,
            Some(d(b"node-x")),
            "edge to orphan removed",
            ParseBackSeverity::Warning,
        );
        assert!(delta.verify_id());
    }

    #[test]
    fn delta_different_kinds_differ() {
        let a = ParseBackTopologyDelta::new(
            TopologyChangeKind::NodeAdded, None, "x", ParseBackSeverity::Info,
        );
        let b = ParseBackTopologyDelta::new(
            TopologyChangeKind::NodeRemoved, None, "x", ParseBackSeverity::Info,
        );
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn delta_critical_flag() {
        let critical = ParseBackTopologyDelta::new(
            TopologyChangeKind::StructuralCrystalAffected,
            None,
            "crystal topology changed",
            ParseBackSeverity::Critical,
        );
        assert!(critical.is_critical());

        let info = ParseBackTopologyDelta::new(
            TopologyChangeKind::NodeAdded, None, "minor", ParseBackSeverity::Info,
        );
        assert!(!info.is_critical());
    }

    #[test]
    fn delta_with_node_id_differs_from_without() {
        let without = ParseBackTopologyDelta::new(
            TopologyChangeKind::NodeModified, None, "mod", ParseBackSeverity::Info,
        );
        let with_id = ParseBackTopologyDelta::new(
            TopologyChangeKind::NodeModified, Some(d(b"n")), "mod", ParseBackSeverity::Info,
        );
        assert_ne!(without.id, with_id.id);
    }

    // --- ParseBackOutcome ---

    #[test]
    fn outcome_passed_variants() {
        assert!(ParseBackOutcome::Passed.is_passed());
        assert!(ParseBackOutcome::TopologyUnchanged.is_passed());
        assert!(!ParseBackOutcome::Failed.is_passed());
    }

    #[test]
    fn outcome_skipped_is_not_failure_class() {
        let o = ParseBackOutcome::SkippedByReportOnly;
        assert!(o.is_skipped_report_only());
        assert!(!o.is_failure_class());
        assert!(!o.is_passed());
    }

    #[test]
    fn outcome_failure_variants() {
        assert!(ParseBackOutcome::Failed.is_failure_class());
        assert!(ParseBackOutcome::Inconclusive.is_failure_class());
    }

    // --- ParseBackPlan ---

    #[test]
    fn plan_id_deterministic() {
        let p1 = basic_plan();
        let p2 = basic_plan();
        assert_eq!(p1.id, p2.id);
    }

    #[test]
    fn plan_verify_id() {
        assert!(basic_plan().verify_id());
    }

    #[test]
    fn plan_different_scope_differs() {
        let p1 = ParseBackPlan::new(
            policy_id(), mat_plan_id(), ParseBackScanScope::AffectedCratesOnly, topo_id(),
        );
        let p2 = ParseBackPlan::new(
            policy_id(), mat_plan_id(), ParseBackScanScope::FullWorkspace, topo_id(),
        );
        assert_ne!(p1.id, p2.id);
    }

    #[test]
    fn plan_with_evidence_changes_id() {
        let plan = basic_plan();
        let id_before = plan.id;
        let plan2 = plan.with_evidence(vec![EvidenceRef::new(
            d(b"ev"),
            EvidenceKind::HostScan,
            "pre-mat scan",
        )]);
        assert_ne!(id_before, plan2.id);
        assert!(plan2.verify_id());
    }

    #[test]
    fn plan_different_baseline_topology_differs() {
        let p1 = basic_plan();
        let p2 = ParseBackPlan::new(
            policy_id(), mat_plan_id(), ParseBackScanScope::AffectedCratesOnly, d(b"other-topo"),
        );
        assert_ne!(p1.id, p2.id);
    }

    // --- ParseBackReport (skipped_by_report_only) ---

    #[test]
    fn report_skipped_deterministic() {
        let r1 = ParseBackReport::skipped_by_report_only(mat_plan_id(), topo_id(), bundle_id());
        let r2 = ParseBackReport::skipped_by_report_only(mat_plan_id(), topo_id(), bundle_id());
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn report_skipped_verify_id() {
        let r = ParseBackReport::skipped_by_report_only(mat_plan_id(), topo_id(), bundle_id());
        assert!(r.verify_id());
        assert!(r.outcome.is_skipped_report_only());
        assert!(r.deltas.is_empty());
    }

    #[test]
    fn report_skipped_pre_post_topology_equal() {
        let r = ParseBackReport::skipped_by_report_only(mat_plan_id(), topo_id(), bundle_id());
        assert_eq!(r.pre_topology_id, r.post_topology_id);
    }

    // --- ParseBackReport (topology_unchanged) ---

    #[test]
    fn report_unchanged_deterministic() {
        let r1 = ParseBackReport::topology_unchanged(mat_plan_id(), topo_id(), bundle_id(), 500);
        let r2 = ParseBackReport::topology_unchanged(mat_plan_id(), topo_id(), bundle_id(), 500);
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn report_unchanged_verify_id() {
        let r = ParseBackReport::topology_unchanged(mat_plan_id(), topo_id(), bundle_id(), 500);
        assert!(r.verify_id());
        assert!(r.outcome.is_passed());
        assert!(r.deltas.is_empty());
    }

    // --- ParseBackReport (new) ---

    #[test]
    fn report_new_deterministic() {
        let delta = ParseBackTopologyDelta::new(
            TopologyChangeKind::NodeAdded, None, "new file", ParseBackSeverity::Info,
        );
        let r1 = ParseBackReport::new(
            mat_plan_id(),
            ParseBackOutcome::Passed,
            vec![delta.clone()],
            topo_id(),
            d(b"post-topo"),
            vec![],
            bundle_id(),
            800,
        );
        let r2 = ParseBackReport::new(
            mat_plan_id(),
            ParseBackOutcome::Passed,
            vec![delta],
            topo_id(),
            d(b"post-topo"),
            vec![],
            bundle_id(),
            800,
        );
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn report_verify_id_with_deltas() {
        let delta = ParseBackTopologyDelta::new(
            TopologyChangeKind::EdgeAdded, None, "new dep", ParseBackSeverity::Warning,
        );
        let r = ParseBackReport::new(
            mat_plan_id(),
            ParseBackOutcome::Passed,
            vec![delta],
            topo_id(),
            d(b"post-topo"),
            vec!["topology expanded".into()],
            bundle_id(),
            1_200,
        );
        assert!(r.verify_id());
    }

    #[test]
    fn report_failed_outcome_on_critical_delta() {
        let delta = ParseBackTopologyDelta::new(
            TopologyChangeKind::StructuralCrystalAffected,
            None,
            "crystal topology broken",
            ParseBackSeverity::Critical,
        );
        let r = ParseBackReport::new(
            mat_plan_id(),
            ParseBackOutcome::Failed,
            vec![delta],
            topo_id(),
            d(b"post-topo"),
            vec!["critical structural violation".into()],
            bundle_id(),
            600,
        );
        assert!(r.outcome.is_failure_class());
        assert!(r.has_critical_delta());
        assert!(r.verify_id());
    }

    #[test]
    fn report_worst_severity_max() {
        let deltas = vec![
            ParseBackTopologyDelta::new(
                TopologyChangeKind::NodeAdded, None, "a", ParseBackSeverity::Info,
            ),
            ParseBackTopologyDelta::new(
                TopologyChangeKind::EdgeRemoved, None, "b", ParseBackSeverity::Critical,
            ),
            ParseBackTopologyDelta::new(
                TopologyChangeKind::NodeModified, None, "c", ParseBackSeverity::Warning,
            ),
        ];
        let r = ParseBackReport::new(
            mat_plan_id(), ParseBackOutcome::Failed, deltas, topo_id(),
            d(b"post"), vec![], bundle_id(), 300,
        );
        assert_eq!(r.worst_severity(), Some(&ParseBackSeverity::Critical));
    }

    #[test]
    fn report_worst_severity_none_when_empty() {
        let r = ParseBackReport::topology_unchanged(mat_plan_id(), topo_id(), bundle_id(), 100);
        assert_eq!(r.worst_severity(), None);
    }

    #[test]
    fn report_different_elapsed_differs() {
        let r1 = ParseBackReport::topology_unchanged(mat_plan_id(), topo_id(), bundle_id(), 100);
        let r2 = ParseBackReport::topology_unchanged(mat_plan_id(), topo_id(), bundle_id(), 200);
        assert_ne!(r1.id, r2.id);
    }

    #[test]
    fn report_evidence_bundle_id_mandatory() {
        let r = ParseBackReport::skipped_by_report_only(mat_plan_id(), topo_id(), bundle_id());
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn report_only_mode_produces_skipped_outcome() {
        use crate::policy::{ImplementationMode, PolicyProfile};
        let profile = PolicyProfile::default();
        assert_eq!(profile.mode, ImplementationMode::ReportOnly);

        let r = ParseBackReport::skipped_by_report_only(mat_plan_id(), topo_id(), bundle_id());
        assert!(r.outcome.is_skipped_report_only());
    }

    #[test]
    fn r2_no_floats_in_elapsed_ms() {
        let r = ParseBackReport::topology_unchanged(mat_plan_id(), topo_id(), bundle_id(), 42_000);
        assert_eq!(r.elapsed_ms, 42_000u64);
    }

    #[test]
    fn r2_evidence_bundle_integration() {
        let ev_bundle = EvidenceBundle::seal(
            vec![EvidenceRef::new(
                d(b"scan-ev"),
                EvidenceKind::HostScan,
                "post-mat scan evidence",
            )],
            policy_id(),
            ReplayStatus::Replayable,
        );
        let r = ParseBackReport::topology_unchanged(
            mat_plan_id(), topo_id(), ev_bundle.bundle_id, 750,
        );
        assert!(r.verify_id());
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
    }
}

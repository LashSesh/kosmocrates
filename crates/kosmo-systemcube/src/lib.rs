//! Kosmocrates SystemCube v0.4.3 — exportable topological blueprint layer.
//!
//! Phase 9 passive export: host snapshot → BlueprintUnits → SystemCubeManifest →
//! D-density report → ContradictionEnergyReport → CompatibilityProfileReport →
//! dry-run KcubeExportReport.
//!
//! No host files are written. Materialization is disabled by default policy.
//! Default mode: `PolicyProfile::default_report_only()`.
//!
//! Key invariants:
//! - SystemCube is NOT raw-code ingestion.
//! - SystemCube is NOT autonomous materialization.
//! - `allow_systemcube_materialization = false` in default `PolicyProfile`.
//! - Blueprint units must be evidence-bound; opaque units are rejected.
//! - D-density and contradiction energy are advisory — they cannot authorise execution.

pub mod blueprint_unit;
pub mod compatibility;
pub mod energy;
pub mod manifest;

pub use blueprint_unit::{BlueprintUnit, BlueprintUnitKind, BlueprintUnitStatus};
pub use compatibility::{CompatibilityGap, CompatibilityProfileReport, CompatibilityStatus};
pub use energy::{ContradictionEnergyReport, ContradictionRecord, EnergyStatus};
pub use manifest::SystemCubeManifest;

use kosmo_core::{
    Digest, KcubeArtifactKind, KcubeExportPolicy, KcubeWriteReport, PolicyProfile, RunDescriptor,
    Q16,
};
use kosmo_kcube::{KcubeArtifact, KcubeExecutor};
use serde::{Deserialize, Serialize};

// ─── D-density Report ─────────────────────────────────────────────────────────

/// Whether D-density could be computed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DDensityStatus {
    /// D-density computed from unit topology.
    Available,
    /// Manifest has no accepted units; density is undefined.
    Unavailable,
}

/// Content for deterministic `DDensityReport` ID.
#[derive(Serialize)]
struct DDensityContent {
    manifest_id: Digest,
    policy_id: Digest,
    status: String,
    density_raw: i64,
    unit_count: u32,
}

/// Advisory D-density report for a `SystemCubeManifest`.
///
/// D-density is a Q16-scaled topological density: accepted_unit_count / total_capacity.
/// It is advisory — high density does not authorise materialization (CROSS-010).
/// When the manifest is empty the status is `Unavailable`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DDensityReport {
    pub report_id: Digest,
    pub manifest_id: Digest,
    pub policy_id: Digest,
    pub status: DDensityStatus,
    /// Q16 density value. 0 = empty; ONE = fully dense.
    pub density: Q16,
    pub accepted_unit_count: u32,
}

impl DDensityReport {
    /// `capacity` = total number of host voids (denominator for D-density).
    pub fn compute(
        manifest: &SystemCubeManifest,
        policy: &PolicyProfile,
        capacity: u32,
    ) -> Self {
        let accepted = manifest.accepted_count() as u32;
        let (status, density) = if accepted == 0 || capacity == 0 {
            (DDensityStatus::Unavailable, Q16::ZERO)
        } else {
            let d = Q16::ratio(accepted as u64, capacity as u64).unwrap_or(Q16::ZERO);
            (DDensityStatus::Available, d)
        };
        let report_id = Digest::of(&DDensityContent {
            manifest_id: manifest.manifest_id,
            policy_id: policy.id,
            status: format!("{:?}", status),
            density_raw: density.raw(),
            unit_count: accepted,
        });
        Self {
            report_id,
            manifest_id: manifest.manifest_id,
            policy_id: policy.id,
            status,
            density,
            accepted_unit_count: accepted,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "DDensityReport — status={:?} density={} units={}",
            self.status,
            self.density.raw(),
            self.accepted_unit_count,
        )
    }
}

// ─── System Cube ──────────────────────────────────────────────────────────────

/// Content for deterministic `SystemCube` ID.
#[derive(Serialize)]
struct SystemCubeContent {
    manifest_id: Digest,
    host_snapshot_id: Digest,
    policy_id: Digest,
}

/// An exportable topological blueprint assembled from evidence-bound `BlueprintUnit`s.
///
/// The `SystemCube` is NOT raw source code and NOT directly executable.
/// It can be exported as a dry-run `.kcube` manifest via `export_dry_run()`.
/// Materialization is disabled by default policy.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemCube {
    pub cube_id: Digest,
    pub manifest: SystemCubeManifest,
    pub host_snapshot_id: Digest,
    pub policy_id: Digest,
    /// All blueprint units (accepted and rejected).
    pub units: Vec<BlueprintUnit>,
}

impl SystemCube {
    pub fn new(
        host_snapshot_id: Digest,
        run: &RunDescriptor,
        policy: &PolicyProfile,
        units: Vec<BlueprintUnit>,
    ) -> Self {
        let manifest = SystemCubeManifest::new(host_snapshot_id, run, policy, &units);
        let cube_id = Digest::of(&SystemCubeContent {
            manifest_id: manifest.manifest_id,
            host_snapshot_id,
            policy_id: policy.id,
        });
        Self {
            cube_id,
            manifest,
            host_snapshot_id,
            policy_id: policy.id,
            units,
        }
    }

    /// Export a dry-run `.kcube` report without writing any host files.
    /// `capacity` is the total number of host voids (used for D-density).
    /// Returns a `KcubeExportReport` — purely advisory, no disk I/O.
    pub fn export_dry_run(
        &self,
        capacity: u32,
        policy: &PolicyProfile,
    ) -> KcubeExportReport {
        let d_density = DDensityReport::compute(&self.manifest, policy, capacity);

        let energy = ContradictionEnergyReport::from_units(
            self.manifest.manifest_id,
            policy,
            &self.units,
        );

        let compatibility = if self.host_snapshot_id == Digest::ZERO {
            CompatibilityProfileReport::no_host_snapshot(self.manifest.manifest_id, policy)
        } else {
            CompatibilityProfileReport::perfect(
                self.manifest.manifest_id,
                self.host_snapshot_id,
                policy,
            )
        };

        KcubeExportReport::new(self, d_density, energy, compatibility, policy)
    }

    /// Write the SystemCube as a real `.kcube` archive via `executor`.
    ///
    /// First performs `export_dry_run` against `op_policy`:
    /// - If `op_policy.allow_systemcube_materialization = false` → returns
    ///   `KcubeWriteReport::skipped_by_report_only` without touching the filesystem.
    /// - Otherwise → serializes the manifest, export assessment, and all accepted
    ///   blueprint units into a `.kcube` archive via `executor.write`.
    ///
    /// `capacity` is forwarded to `export_dry_run` for D-density computation.
    /// `evidence_bundle_id` is bound to every produced report (CROSS-006).
    pub fn export_to_kcube(
        &self,
        executor: &KcubeExecutor,
        capacity: u32,
        export_policy: &KcubeExportPolicy,
        op_policy: &PolicyProfile,
        evidence_bundle_id: Digest,
        sequence: u64,
    ) -> KcubeWriteReport {
        let dry_run = self.export_dry_run(capacity, op_policy);

        if dry_run.mode == KcubeExportMode::BlockedByPolicy {
            return KcubeWriteReport::skipped_by_report_only(
                Digest::ZERO,
                export_policy.id,
                evidence_bundle_id,
            );
        }

        let artifacts = self.to_kcube_artifacts(&dry_run);
        let scope = format!("systemcube-{}", &self.cube_id.to_hex()[..16]);
        executor.write(&scope, artifacts, export_policy, evidence_bundle_id, sequence)
    }

    /// Serializes the cube's manifest, dry-run export report, and accepted
    /// blueprint units into a `Vec<KcubeArtifact>` ready for `KcubeExecutor::write`.
    fn to_kcube_artifacts(&self, dry_run: &KcubeExportReport) -> Vec<KcubeArtifact> {
        let mut artifacts = Vec::new();

        // Canonical manifest descriptor
        let manifest_bytes = serde_json::to_vec(&self.manifest)
            .expect("SystemCubeManifest is always serializable");
        artifacts.push(KcubeArtifact::new(
            KcubeArtifactKind::CartographyManifest,
            "manifest.json",
            manifest_bytes,
        ));

        // Dry-run export assessment (advisory; included for self-documentation)
        let report_bytes = serde_json::to_vec(dry_run)
            .expect("KcubeExportReport is always serializable");
        artifacts.push(KcubeArtifact::new(
            KcubeArtifactKind::ValidationClosureReport,
            "export_report.json",
            report_bytes,
        ));

        // Accepted blueprint units — one JSON file each, keyed by unit_id
        for unit in self.units.iter().filter(|u| u.is_accepted()) {
            let unit_bytes = serde_json::to_vec(unit)
                .expect("BlueprintUnit is always serializable");
            let path = format!("units/{}.json", unit.unit_id.to_hex());
            artifacts.push(KcubeArtifact::new(
                KcubeArtifactKind::StructuralCrystal,
                path,
                unit_bytes,
            ));
        }

        artifacts
    }
}

// ─── Kcube Export Report ──────────────────────────────────────────────────────

/// Export mode for a `.kcube` dry-run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KcubeExportMode {
    /// Dry-run: report produced, no file written.
    DryRun,
    /// Blocked by policy (materialization not permitted).
    BlockedByPolicy,
}

/// Content for deterministic `KcubeExportReport` ID.
#[derive(Serialize)]
struct ExportReportContent {
    cube_id: Digest,
    manifest_id: Digest,
    policy_id: Digest,
    mode: String,
    d_density_id: Digest,
    energy_id: Digest,
    compatibility_id: Digest,
}

/// The result of a dry-run `.kcube` export from a `SystemCube`.
///
/// Bundles the manifest, D-density, contradiction energy, and compatibility
/// reports into a single content-addressed artifact.
///
/// No disk I/O is performed. `mode` is always `DryRun` in Phase 9.
/// Materialization requires an explicit `PolicyProfile` change and operator approval.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcubeExportReport {
    pub export_id: Digest,
    pub cube_id: Digest,
    pub manifest_id: Digest,
    pub policy_id: Digest,
    pub mode: KcubeExportMode,
    pub d_density: DDensityReport,
    pub contradiction_energy: ContradictionEnergyReport,
    pub compatibility: CompatibilityProfileReport,
}

impl KcubeExportReport {
    fn new(
        cube: &SystemCube,
        d_density: DDensityReport,
        contradiction_energy: ContradictionEnergyReport,
        compatibility: CompatibilityProfileReport,
        policy: &PolicyProfile,
    ) -> Self {
        let mode = if policy.allow_systemcube_materialization {
            KcubeExportMode::DryRun
        } else {
            KcubeExportMode::BlockedByPolicy
        };
        let export_id = Digest::of(&ExportReportContent {
            cube_id: cube.cube_id,
            manifest_id: cube.manifest.manifest_id,
            policy_id: policy.id,
            mode: format!("{:?}", mode),
            d_density_id: d_density.report_id,
            energy_id: contradiction_energy.report_id,
            compatibility_id: compatibility.report_id,
        });
        Self {
            export_id,
            cube_id: cube.cube_id,
            manifest_id: cube.manifest.manifest_id,
            policy_id: policy.id,
            mode,
            d_density,
            contradiction_energy,
            compatibility,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "KcubeExportReport — mode={:?} manifest={:.8} | {} | {} | {}",
            self.mode,
            hex::to_hex(&self.manifest_id.as_bytes()[..4]),
            self.d_density.summary(),
            self.contradiction_energy.summary(),
            self.compatibility.summary(),
        )
    }
}

mod hex {
    pub fn to_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprint_unit::{BlueprintUnit, BlueprintUnitKind};
    use kosmo_core::{AuthorityLabel, PolicyProfile, RunDescriptor, TaintLabel};

    fn policy() -> PolicyProfile {
        PolicyProfile::default_report_only()
    }

    fn run() -> RunDescriptor {
        RunDescriptor::new(policy().id, "host")
    }

    fn host_snap() -> Digest {
        Digest::of_bytes(b"host")
    }

    fn make_unit(src_tag: &[u8]) -> BlueprintUnit {
        BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            Digest::of_bytes(src_tag),
            AuthorityLabel::Operator,
            TaintLabel::Clean,
            vec![Digest::of_bytes(b"ev")],
            &policy(),
        )
    }

    fn make_cube(units: Vec<BlueprintUnit>) -> SystemCube {
        SystemCube::new(host_snap(), &run(), &policy(), units)
    }

    // ── D-density Report ──────────────────────────────────────────────────────

    #[test]
    fn d_density_is_content_addressed() {
        let cube = make_cube(vec![make_unit(b"u1")]);
        let r1 = DDensityReport::compute(&cube.manifest, &policy(), 4);
        let r2 = DDensityReport::compute(&cube.manifest, &policy(), 4);
        assert_eq!(r1.report_id, r2.report_id);
        assert_ne!(r1.report_id, Digest::ZERO);
    }

    #[test]
    fn d_density_available_for_non_empty_manifest() {
        let cube = make_cube(vec![make_unit(b"u1"), make_unit(b"u2")]);
        let r = DDensityReport::compute(&cube.manifest, &policy(), 4);
        assert_eq!(r.status, DDensityStatus::Available);
        assert!(r.density.raw() > 0, "density must be positive");
    }

    #[test]
    fn d_density_unavailable_for_empty_manifest() {
        let cube = make_cube(vec![]);
        let r = DDensityReport::compute(&cube.manifest, &policy(), 4);
        assert_eq!(r.status, DDensityStatus::Unavailable);
        assert_eq!(r.density, Q16::ZERO);
    }

    #[test]
    fn d_density_full_when_capacity_equals_units() {
        let cube = make_cube(vec![make_unit(b"u1"), make_unit(b"u2")]);
        let r = DDensityReport::compute(&cube.manifest, &policy(), 2);
        assert_eq!(r.status, DDensityStatus::Available);
        assert_eq!(r.density, Q16::ONE, "2/2 = 1.0");
    }

    #[test]
    fn d_density_summary_non_empty() {
        let cube = make_cube(vec![make_unit(b"u1")]);
        let r = DDensityReport::compute(&cube.manifest, &policy(), 4);
        let s = r.summary();
        assert!(s.contains("DDensityReport"));
    }

    // ── SystemCube ────────────────────────────────────────────────────────────

    #[test]
    fn system_cube_is_content_addressed() {
        let u = make_unit(b"u1");
        let c1 = make_cube(vec![u.clone()]);
        let c2 = make_cube(vec![u]);
        assert_eq!(c1.cube_id, c2.cube_id);
        assert_ne!(c1.cube_id, Digest::ZERO);
    }

    #[test]
    fn system_cube_manifest_excludes_rejected_units() {
        let accepted = make_unit(b"u_accepted");
        let rejected = BlueprintUnit::new(
            BlueprintUnitKind::FiberDescriptor,
            Digest::of_bytes(b"opaque"),
            AuthorityLabel::Operator,
            TaintLabel::Clean,
            vec![], // no evidence
            &policy(),
        );
        let cube = make_cube(vec![accepted, rejected]);
        assert_eq!(cube.manifest.accepted_count(), 1);
        assert_eq!(cube.manifest.rejected_unit_count, 1);
    }

    // ── Dry-run Export ────────────────────────────────────────────────────────

    #[test]
    fn kcube_export_report_is_content_addressed() {
        let cube = make_cube(vec![make_unit(b"u1")]);
        let r1 = cube.export_dry_run(4, &policy());
        let r2 = cube.export_dry_run(4, &policy());
        assert_eq!(r1.export_id, r2.export_id);
        assert_ne!(r1.export_id, Digest::ZERO);
    }

    #[test]
    fn kcube_export_blocked_by_default_policy() {
        let cube = make_cube(vec![make_unit(b"u1")]);
        let r = cube.export_dry_run(4, &policy());
        assert_eq!(
            r.mode,
            KcubeExportMode::BlockedByPolicy,
            "default ReportOnly policy must block materialization"
        );
    }

    #[test]
    fn kcube_export_manifest_digest_round_trips() {
        let cube = make_cube(vec![make_unit(b"u1")]);
        let r = cube.export_dry_run(4, &policy());
        // Manifest ID in export must match the manifest inside the cube.
        assert_eq!(r.manifest_id, cube.manifest.manifest_id);
    }

    #[test]
    fn kcube_export_summary_non_empty() {
        let cube = make_cube(vec![make_unit(b"u1")]);
        let r = cube.export_dry_run(4, &policy());
        let s = r.summary();
        assert!(!s.is_empty());
        assert!(s.contains("KcubeExportReport"));
    }

    #[test]
    fn kcube_export_d_density_present() {
        let cube = make_cube(vec![make_unit(b"u1"), make_unit(b"u2")]);
        let r = cube.export_dry_run(4, &policy());
        assert_eq!(r.d_density.status, DDensityStatus::Available);
    }

    #[test]
    fn kcube_export_energy_insufficient_for_single_unit() {
        let cube = make_cube(vec![make_unit(b"u1")]);
        let r = cube.export_dry_run(4, &policy());
        assert_eq!(r.contradiction_energy.status, EnergyStatus::Insufficient);
    }

    #[test]
    fn kcube_export_compatibility_no_host_snapshot_when_zero_id() {
        // Build a cube with zero host_snapshot_id.
        let manifest = SystemCubeManifest::new(Digest::ZERO, &run(), &policy(), &[make_unit(b"u1")]);
        let cube_id = Digest::of(&SystemCubeContent {
            manifest_id: manifest.manifest_id,
            host_snapshot_id: Digest::ZERO,
            policy_id: policy().id,
        });
        let cube = SystemCube {
            cube_id,
            manifest,
            host_snapshot_id: Digest::ZERO,
            policy_id: policy().id,
            units: vec![make_unit(b"u1")],
        };
        let r = cube.export_dry_run(4, &policy());
        assert_eq!(r.compatibility.status, CompatibilityStatus::NoHostSnapshot);
    }

    // ── CROSS-010: D-density does not bypass gates ────────────────────────────

    #[test]
    fn cross_010_d_density_high_does_not_authorise_materialization() {
        let cube = make_cube(vec![make_unit(b"u1"), make_unit(b"u2")]);
        let r = cube.export_dry_run(2, &policy());
        // Density is 1.0 (full) but mode must still be BlockedByPolicy.
        assert_eq!(r.d_density.density, Q16::ONE, "density should be 1.0");
        assert_eq!(
            r.mode,
            KcubeExportMode::BlockedByPolicy,
            "full D-density must NOT bypass policy gate"
        );
    }

    // ── CROSS-013: no host files written ──────────────────────────────────────

    #[test]
    fn cross_013_export_dry_run_has_no_write_interface() {
        // Structural test: KcubeExportReport has no write/patch/mutation method.
        let cube = make_cube(vec![make_unit(b"u1")]);
        let r = cube.export_dry_run(4, &policy());
        // The only callable method is summary(). Policy forbids host writes.
        let _s = r.summary();
        assert!(!policy().allow_host_write);
        assert!(!policy().allow_systemcube_materialization);
    }

    // ── export_to_kcube weld tests ─────────────────────────────────────────

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join("kosmo-systemcube-tests").join(tag);
        std::fs::remove_dir_all(&p).ok();
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn export_policy_full() -> KcubeExportPolicy {
        KcubeExportPolicy::write_once(
            Digest::of_bytes(b"pol"),
            Digest::of_bytes(b"dir"),
            vec![
                KcubeArtifactKind::CartographyManifest,
                KcubeArtifactKind::ValidationClosureReport,
                KcubeArtifactKind::StructuralCrystal,
            ],
        )
    }

    #[test]
    fn export_to_kcube_blocked_when_materialization_disabled() {
        let dir = tmp_dir("blocked");
        let exec = KcubeExecutor::new(&dir);
        let cube = make_cube(vec![make_unit(b"u1")]);
        // default policy has allow_systemcube_materialization = false
        let r = cube.export_to_kcube(&exec, 4, &export_policy_full(), &policy(), Digest::of_bytes(b"ev"), 1);
        assert!(
            r.outcome.is_skipped_report_only(),
            "blocked export must be SkippedByReportOnly, got {:?}", r.outcome
        );
        assert_eq!(r.written_bytes, 0);
        assert!(r.verify_id());
    }

    #[test]
    fn export_to_kcube_writes_archive_when_allowed() {
        let dir = tmp_dir("allowed");
        let exec = KcubeExecutor::new(&dir);
        let cube = make_cube(vec![make_unit(b"u1"), make_unit(b"u2")]);
        let op = PolicyProfile::operator_approved_with_systemcube();
        let r = cube.export_to_kcube(&exec, 4, &export_policy_full(), &op, Digest::of_bytes(b"ev"), 1);
        assert!(r.outcome.is_written(), "export must succeed, got {:?}", r.outcome);
        assert!(r.roundtrip_passed(), "roundtrip must pass");
        assert!(r.written_bytes > 0);
        assert!(r.verify_id());
    }

    #[test]
    fn export_to_kcube_archive_parses_back() {
        let dir = tmp_dir("parses-back");
        let exec = KcubeExecutor::new(&dir);
        let cube = make_cube(vec![make_unit(b"u1")]);
        let op = PolicyProfile::operator_approved_with_systemcube();
        let r = cube.export_to_kcube(&exec, 4, &export_policy_full(), &op, Digest::of_bytes(b"ev"), 7);
        assert!(r.outcome.is_written(), "{:?}", r.outcome);
        // The scope is derived from cube_id hex; sequence = 7
        let scope = format!("systemcube-{}", &cube.cube_id.to_hex()[..16]);
        use kosmo_kcube::kcube_file_name;
        let fname = kcube_file_name(&scope, 7);
        let pkg = exec.read(&fname).expect("read must succeed");
        assert!(pkg.verify_id());
        // Manifest + export_report + 1 accepted unit = 3 artifacts
        assert_eq!(pkg.entry_count(), 3, "expected 3 artifacts (manifest, report, 1 unit)");
    }

    #[test]
    fn export_to_kcube_report_evidence_bound() {
        let dir = tmp_dir("evidence-bound");
        let exec = KcubeExecutor::new(&dir);
        let cube = make_cube(vec![]);
        // No units → export succeeds with manifest + report only (0 crystal artifacts)
        let op = PolicyProfile::operator_approved_with_systemcube();
        let r = cube.export_to_kcube(&exec, 0, &export_policy_full(), &op, Digest::of_bytes(b"ev"), 1);
        assert_ne!(r.evidence_bundle_id, Digest::ZERO, "CROSS-006: evidence must be non-zero");
        assert!(r.verify_id());
    }

    #[test]
    fn operator_approved_with_systemcube_policy_verify_id() {
        let p = PolicyProfile::operator_approved_with_systemcube();
        assert!(p.allow_systemcube_materialization);
        assert!(p.allow_host_write);
        assert!(p.verify_id());
    }
}

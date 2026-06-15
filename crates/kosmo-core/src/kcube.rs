use crate::digest::Digest;
use serde::{Deserialize, Serialize};

/// Classifies the kind of artifact included in a `.kcube` package.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KcubeArtifactKind {
    StructuralCrystal,
    EvidenceBundle,
    ValidationClosureReport,
    MaterializationExecutionReport,
    CartographyManifest,
    PolicyProfile,
    FoundryExecutionReport,
    ParseBackReport,
    Custom(String),
}

/// A single artifact entry in a `KcubePackage` manifest.
///
/// `id` is the content digest of all fields except `id`.
/// `size_bytes` is `u64` — no floats in any audit path.
/// `path` is a relative path within the `.kcube` archive.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KcubeManifestEntry {
    pub id: Digest,
    pub artifact_digest: Digest,
    pub artifact_kind: KcubeArtifactKind,
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Serialize)]
struct ManifestEntryContent<'a> {
    artifact_digest: &'a Digest,
    artifact_kind: &'a KcubeArtifactKind,
    path: &'a String,
    size_bytes: u64,
}

impl KcubeManifestEntry {
    pub fn new(
        artifact_digest: Digest,
        artifact_kind: KcubeArtifactKind,
        path: impl Into<String>,
        size_bytes: u64,
    ) -> Self {
        let mut e = Self {
            id: Digest::ZERO,
            artifact_digest,
            artifact_kind,
            path: path.into(),
            size_bytes,
        };
        e.id = e.compute_id();
        e
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&ManifestEntryContent {
            artifact_digest: &self.artifact_digest,
            artifact_kind: &self.artifact_kind,
            path: &self.path,
            size_bytes: self.size_bytes,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// A deterministic, content-addressed `.kcube` package descriptor.
///
/// `package_digest` is the digest of the full archive byte content
/// (computed externally and bound here). `id` is the digest of the
/// descriptor fields — separate from `package_digest` so the descriptor
/// can be verified without re-reading the archive.
///
/// `created_at_sequence` is a monotonic sequence counter — no wall-clock
/// timestamps, which are non-deterministic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KcubePackage {
    pub id: Digest,
    /// Digest of the full `.kcube` archive bytes.
    pub package_digest: Digest,
    pub manifest: Vec<KcubeManifestEntry>,
    pub scope: String,
    pub policy_id: Digest,
    pub evidence_bundle_id: Digest,
    /// Monotonic sequence counter — used instead of a wall-clock timestamp.
    pub created_at_sequence: u64,
}

#[derive(Serialize)]
struct PackageContent<'a> {
    package_digest: &'a Digest,
    entry_ids: Vec<&'a Digest>,
    scope: &'a String,
    policy_id: &'a Digest,
    evidence_bundle_id: &'a Digest,
    created_at_sequence: u64,
}

impl KcubePackage {
    pub fn new(
        package_digest: Digest,
        manifest: Vec<KcubeManifestEntry>,
        scope: impl Into<String>,
        policy_id: Digest,
        evidence_bundle_id: Digest,
        created_at_sequence: u64,
    ) -> Self {
        let mut p = Self {
            id: Digest::ZERO,
            package_digest,
            manifest,
            scope: scope.into(),
            policy_id,
            evidence_bundle_id,
            created_at_sequence,
        };
        p.id = p.compute_id();
        p
    }

    fn compute_id(&self) -> Digest {
        let entry_ids: Vec<&Digest> = self.manifest.iter().map(|e| &e.id).collect();
        Digest::of(&PackageContent {
            package_digest: &self.package_digest,
            entry_ids,
            scope: &self.scope,
            policy_id: &self.policy_id,
            evidence_bundle_id: &self.evidence_bundle_id,
            created_at_sequence: self.created_at_sequence,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    pub fn total_size_bytes(&self) -> u64 {
        self.manifest.iter().map(|e| e.size_bytes).sum()
    }

    pub fn entry_count(&self) -> usize {
        self.manifest.len()
    }
}

/// Governs what a `.kcube` export operation may do.
///
/// `allow_write` is `false` in `ReportOnly` mode — write is forbidden.
/// `allow_overwrite` defaults to `false` — existing packages are never silently
/// replaced.
/// `require_roundtrip_verification` defaults to `true` — written bytes must
/// be re-read and their digest verified before the write report is issued.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KcubeExportPolicy {
    pub id: Digest,
    pub policy_id: Digest,
    pub allow_write: bool,
    pub allow_overwrite: bool,
    /// Digest of the directory path where the `.kcube` file will be written.
    pub target_dir_digest: Digest,
    pub allowed_artifact_kinds: Vec<KcubeArtifactKind>,
    pub require_roundtrip_verification: bool,
}

#[derive(Serialize)]
struct ExportPolicyContent<'a> {
    policy_id: &'a Digest,
    allow_write: bool,
    allow_overwrite: bool,
    target_dir_digest: &'a Digest,
    allowed_artifact_kinds: &'a Vec<KcubeArtifactKind>,
    require_roundtrip_verification: bool,
}

impl KcubeExportPolicy {
    /// Fail-closed default: write forbidden, overwrite forbidden,
    /// roundtrip verification required.
    pub fn report_only(policy_id: Digest, target_dir_digest: Digest) -> Self {
        let mut p = Self {
            id: Digest::ZERO,
            policy_id,
            allow_write: false,
            allow_overwrite: false,
            target_dir_digest,
            allowed_artifact_kinds: vec![],
            require_roundtrip_verification: true,
        };
        p.id = p.compute_id();
        p
    }

    /// Export policy that permits writing but not overwriting.
    pub fn write_once(
        policy_id: Digest,
        target_dir_digest: Digest,
        allowed_artifact_kinds: Vec<KcubeArtifactKind>,
    ) -> Self {
        let mut p = Self {
            id: Digest::ZERO,
            policy_id,
            allow_write: true,
            allow_overwrite: false,
            target_dir_digest,
            allowed_artifact_kinds,
            require_roundtrip_verification: true,
        };
        p.id = p.compute_id();
        p
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&ExportPolicyContent {
            policy_id: &self.policy_id,
            allow_write: self.allow_write,
            allow_overwrite: self.allow_overwrite,
            target_dir_digest: &self.target_dir_digest,
            allowed_artifact_kinds: &self.allowed_artifact_kinds,
            require_roundtrip_verification: self.require_roundtrip_verification,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    pub fn is_artifact_kind_allowed(&self, kind: &KcubeArtifactKind) -> bool {
        self.allowed_artifact_kinds.contains(kind)
    }
}

/// Verifies that a written `.kcube` archive can be read back with the
/// expected digest (roundtrip integrity check).
///
/// `verification_passed` is `true` iff `expected_package_digest == observed_package_digest`.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KcubeRoundtripVerification {
    pub id: Digest,
    pub expected_package_digest: Digest,
    pub observed_package_digest: Digest,
    pub verification_passed: bool,
    pub evidence_bundle_id: Digest,
}

#[derive(Serialize)]
struct RoundtripContent<'a> {
    expected_package_digest: &'a Digest,
    observed_package_digest: &'a Digest,
    verification_passed: bool,
    evidence_bundle_id: &'a Digest,
}

impl KcubeRoundtripVerification {
    pub fn new(
        expected_package_digest: Digest,
        observed_package_digest: Digest,
        evidence_bundle_id: Digest,
    ) -> Self {
        let verification_passed = expected_package_digest == observed_package_digest;
        let mut v = Self {
            id: Digest::ZERO,
            expected_package_digest,
            observed_package_digest,
            verification_passed,
            evidence_bundle_id,
        };
        v.id = v.compute_id();
        v
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&RoundtripContent {
            expected_package_digest: &self.expected_package_digest,
            observed_package_digest: &self.observed_package_digest,
            verification_passed: self.verification_passed,
            evidence_bundle_id: &self.evidence_bundle_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Outcome of a `.kcube` write operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KcubeWriteOutcome {
    /// Skipped because `PolicyProfile.mode == ReportOnly`.
    SkippedByReportOnly,
    /// Archive written and roundtrip verification passed.
    Written,
    /// Archive written but roundtrip verification failed.
    WrittenRoundtripFailed,
    /// Write operation failed.
    Failed { reason: String },
    /// Export policy denied the write.
    DeniedByPolicy { reason: String },
}

impl KcubeWriteOutcome {
    pub fn is_written(&self) -> bool {
        matches!(self, Self::Written)
    }

    pub fn is_skipped_report_only(&self) -> bool {
        matches!(self, Self::SkippedByReportOnly)
    }

    pub fn is_failure_class(&self) -> bool {
        matches!(
            self,
            Self::WrittenRoundtripFailed | Self::Failed { .. } | Self::DeniedByPolicy { .. }
        )
    }
}

/// The complete, content-addressed record of a `.kcube` export write operation.
///
/// `roundtrip` is `None` when the write was skipped or failed before verification.
/// `written_bytes` is `u64` — no floats in any audit path.
/// `evidence_bundle_id` is mandatory.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KcubeWriteReport {
    pub id: Digest,
    pub package_id: Digest,
    pub export_policy_id: Digest,
    pub outcome: KcubeWriteOutcome,
    /// Digest of the path where the `.kcube` file was written; `None` if not written.
    pub written_path_digest: Option<Digest>,
    pub written_bytes: u64,
    pub roundtrip: Option<KcubeRoundtripVerification>,
    pub diagnostics: Vec<String>,
    pub evidence_bundle_id: Digest,
    pub elapsed_ms: u64,
}

#[derive(Serialize)]
struct WriteReportContent<'a> {
    package_id: &'a Digest,
    export_policy_id: &'a Digest,
    outcome: &'a KcubeWriteOutcome,
    written_path_digest: &'a Option<Digest>,
    written_bytes: u64,
    roundtrip_id: Option<&'a Digest>,
    diagnostics: &'a Vec<String>,
    evidence_bundle_id: &'a Digest,
}

impl KcubeWriteReport {
    pub fn skipped_by_report_only(
        package_id: Digest,
        export_policy_id: Digest,
        evidence_bundle_id: Digest,
    ) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            package_id,
            export_policy_id,
            outcome: KcubeWriteOutcome::SkippedByReportOnly,
            written_path_digest: None,
            written_bytes: 0,
            roundtrip: None,
            diagnostics: vec!["Skipped: PolicyProfile.mode == ReportOnly".into()],
            evidence_bundle_id,
            elapsed_ms: 0,
        };
        r.id = r.compute_id();
        r
    }

    pub fn new(
        package_id: Digest,
        export_policy_id: Digest,
        outcome: KcubeWriteOutcome,
        written_path_digest: Option<Digest>,
        written_bytes: u64,
        roundtrip: Option<KcubeRoundtripVerification>,
        diagnostics: Vec<String>,
        evidence_bundle_id: Digest,
        elapsed_ms: u64,
    ) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            package_id,
            export_policy_id,
            outcome,
            written_path_digest,
            written_bytes,
            roundtrip,
            diagnostics,
            evidence_bundle_id,
            elapsed_ms,
        };
        r.id = r.compute_id();
        r
    }

    fn compute_id(&self) -> Digest {
        let roundtrip_id = self.roundtrip.as_ref().map(|r| &r.id);
        Digest::of(&WriteReportContent {
            package_id: &self.package_id,
            export_policy_id: &self.export_policy_id,
            outcome: &self.outcome,
            written_path_digest: &self.written_path_digest,
            written_bytes: self.written_bytes,
            roundtrip_id,
            diagnostics: &self.diagnostics,
            evidence_bundle_id: &self.evidence_bundle_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    pub fn roundtrip_passed(&self) -> bool {
        self.roundtrip
            .as_ref()
            .is_some_and(|r| r.verification_passed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{EvidenceBundle, EvidenceKind, EvidenceRef, ReplayStatus};

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn policy_id() -> Digest {
        d(b"policy")
    }
    fn bundle_id() -> Digest {
        d(b"bundle")
    }
    fn pkg_digest() -> Digest {
        d(b"pkg-bytes")
    }
    fn target_dir() -> Digest {
        d(b"target-dir")
    }

    fn basic_entry(path: &str) -> KcubeManifestEntry {
        KcubeManifestEntry::new(
            d(path.as_bytes()),
            KcubeArtifactKind::StructuralCrystal,
            path,
            1_024,
        )
    }

    fn basic_package() -> KcubePackage {
        KcubePackage::new(
            pkg_digest(),
            vec![
                basic_entry("crystals/c1.bin"),
                basic_entry("crystals/c2.bin"),
            ],
            "kosmo-core-v0.1",
            policy_id(),
            bundle_id(),
            1,
        )
    }

    fn report_only_export_policy() -> KcubeExportPolicy {
        KcubeExportPolicy::report_only(policy_id(), target_dir())
    }

    // --- KcubeManifestEntry ---

    #[test]
    fn entry_id_deterministic() {
        let e1 = basic_entry("a.bin");
        let e2 = basic_entry("a.bin");
        assert_eq!(e1.id, e2.id);
    }

    #[test]
    fn entry_verify_id() {
        assert!(basic_entry("a.bin").verify_id());
    }

    #[test]
    fn entry_different_paths_differ() {
        let e1 = basic_entry("a.bin");
        let e2 = basic_entry("b.bin");
        assert_ne!(e1.id, e2.id);
    }

    #[test]
    fn entry_different_size_differs() {
        let e1 = KcubeManifestEntry::new(d(b"x"), KcubeArtifactKind::EvidenceBundle, "x", 100);
        let e2 = KcubeManifestEntry::new(d(b"x"), KcubeArtifactKind::EvidenceBundle, "x", 200);
        assert_ne!(e1.id, e2.id);
    }

    #[test]
    fn entry_different_kinds_differ() {
        let e1 = KcubeManifestEntry::new(d(b"x"), KcubeArtifactKind::StructuralCrystal, "x", 100);
        let e2 = KcubeManifestEntry::new(d(b"x"), KcubeArtifactKind::EvidenceBundle, "x", 100);
        assert_ne!(e1.id, e2.id);
    }

    // --- KcubePackage ---

    #[test]
    fn package_id_deterministic() {
        let p1 = basic_package();
        let p2 = basic_package();
        assert_eq!(p1.id, p2.id);
    }

    #[test]
    fn package_verify_id() {
        assert!(basic_package().verify_id());
    }

    #[test]
    fn package_total_size() {
        let p = basic_package();
        assert_eq!(p.total_size_bytes(), 2_048);
    }

    #[test]
    fn package_entry_count() {
        assert_eq!(basic_package().entry_count(), 2);
    }

    #[test]
    fn package_different_sequence_differs() {
        let p1 = KcubePackage::new(pkg_digest(), vec![], "scope", policy_id(), bundle_id(), 1);
        let p2 = KcubePackage::new(pkg_digest(), vec![], "scope", policy_id(), bundle_id(), 2);
        assert_ne!(p1.id, p2.id);
    }

    #[test]
    fn package_different_scope_differs() {
        let p1 = KcubePackage::new(pkg_digest(), vec![], "scope-a", policy_id(), bundle_id(), 1);
        let p2 = KcubePackage::new(pkg_digest(), vec![], "scope-b", policy_id(), bundle_id(), 1);
        assert_ne!(p1.id, p2.id);
    }

    #[test]
    fn package_no_wall_clock_timestamp() {
        // created_at_sequence is u64 monotonic counter — deterministic.
        let p = basic_package();
        assert_eq!(p.created_at_sequence, 1u64);
    }

    // --- KcubeExportPolicy ---

    #[test]
    fn export_policy_report_only_id_deterministic() {
        let p1 = report_only_export_policy();
        let p2 = report_only_export_policy();
        assert_eq!(p1.id, p2.id);
    }

    #[test]
    fn export_policy_report_only_verify_id() {
        assert!(report_only_export_policy().verify_id());
    }

    #[test]
    fn export_policy_report_only_denies_write() {
        let p = report_only_export_policy();
        assert!(!p.allow_write);
        assert!(!p.allow_overwrite);
        assert!(p.require_roundtrip_verification);
    }

    #[test]
    fn export_policy_write_once_allows_write() {
        let p = KcubeExportPolicy::write_once(
            policy_id(),
            target_dir(),
            vec![KcubeArtifactKind::StructuralCrystal],
        );
        assert!(p.allow_write);
        assert!(!p.allow_overwrite);
        assert!(p.require_roundtrip_verification);
        assert!(p.verify_id());
    }

    #[test]
    fn export_policy_artifact_kind_check() {
        let p = KcubeExportPolicy::write_once(
            policy_id(),
            target_dir(),
            vec![
                KcubeArtifactKind::StructuralCrystal,
                KcubeArtifactKind::EvidenceBundle,
            ],
        );
        assert!(p.is_artifact_kind_allowed(&KcubeArtifactKind::StructuralCrystal));
        assert!(p.is_artifact_kind_allowed(&KcubeArtifactKind::EvidenceBundle));
        assert!(!p.is_artifact_kind_allowed(&KcubeArtifactKind::PolicyProfile));
    }

    #[test]
    fn export_policy_report_only_allows_nothing() {
        let p = report_only_export_policy();
        assert!(!p.is_artifact_kind_allowed(&KcubeArtifactKind::StructuralCrystal));
    }

    // --- KcubeRoundtripVerification ---

    #[test]
    fn roundtrip_passed_when_digests_match() {
        let digest = d(b"archive");
        let v = KcubeRoundtripVerification::new(digest, digest, bundle_id());
        assert!(v.verification_passed);
        assert!(v.verify_id());
    }

    #[test]
    fn roundtrip_failed_when_digests_differ() {
        let v = KcubeRoundtripVerification::new(d(b"expected"), d(b"observed"), bundle_id());
        assert!(!v.verification_passed);
        assert!(v.verify_id());
    }

    #[test]
    fn roundtrip_id_deterministic() {
        let v1 = KcubeRoundtripVerification::new(d(b"a"), d(b"a"), bundle_id());
        let v2 = KcubeRoundtripVerification::new(d(b"a"), d(b"a"), bundle_id());
        assert_eq!(v1.id, v2.id);
    }

    #[test]
    fn roundtrip_different_observed_differs() {
        let v1 = KcubeRoundtripVerification::new(d(b"exp"), d(b"obs-a"), bundle_id());
        let v2 = KcubeRoundtripVerification::new(d(b"exp"), d(b"obs-b"), bundle_id());
        assert_ne!(v1.id, v2.id);
    }

    // --- KcubeWriteOutcome ---

    #[test]
    fn write_outcome_written_is_not_failure() {
        assert!(KcubeWriteOutcome::Written.is_written());
        assert!(!KcubeWriteOutcome::Written.is_failure_class());
    }

    #[test]
    fn write_outcome_skipped_is_not_failure() {
        let o = KcubeWriteOutcome::SkippedByReportOnly;
        assert!(o.is_skipped_report_only());
        assert!(!o.is_failure_class());
        assert!(!o.is_written());
    }

    #[test]
    fn write_outcome_failure_variants() {
        assert!(KcubeWriteOutcome::WrittenRoundtripFailed.is_failure_class());
        assert!(KcubeWriteOutcome::Failed {
            reason: "io".into()
        }
        .is_failure_class());
        assert!(KcubeWriteOutcome::DeniedByPolicy {
            reason: "mode".into()
        }
        .is_failure_class());
    }

    // --- KcubeWriteReport ---

    #[test]
    fn write_report_skipped_deterministic() {
        let r1 = KcubeWriteReport::skipped_by_report_only(d(b"pkg"), d(b"ep"), bundle_id());
        let r2 = KcubeWriteReport::skipped_by_report_only(d(b"pkg"), d(b"ep"), bundle_id());
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn write_report_skipped_verify_id() {
        let r = KcubeWriteReport::skipped_by_report_only(d(b"pkg"), d(b"ep"), bundle_id());
        assert!(r.verify_id());
        assert!(r.outcome.is_skipped_report_only());
        assert!(r.written_path_digest.is_none());
        assert!(r.roundtrip.is_none());
        assert_eq!(r.written_bytes, 0);
    }

    #[test]
    fn write_report_with_roundtrip_passed() {
        let rt = KcubeRoundtripVerification::new(pkg_digest(), pkg_digest(), bundle_id());
        let r = KcubeWriteReport::new(
            d(b"pkg"),
            d(b"ep"),
            KcubeWriteOutcome::Written,
            Some(d(b"out-path")),
            4_096,
            Some(rt),
            vec![],
            bundle_id(),
            250,
        );
        assert!(r.verify_id());
        assert!(r.outcome.is_written());
        assert!(r.roundtrip_passed());
    }

    #[test]
    fn write_report_roundtrip_failed_outcome() {
        let rt = KcubeRoundtripVerification::new(d(b"expected"), d(b"different"), bundle_id());
        let r = KcubeWriteReport::new(
            d(b"pkg"),
            d(b"ep"),
            KcubeWriteOutcome::WrittenRoundtripFailed,
            Some(d(b"out-path")),
            4_096,
            Some(rt),
            vec!["roundtrip digest mismatch".into()],
            bundle_id(),
            300,
        );
        assert!(r.verify_id());
        assert!(r.outcome.is_failure_class());
        assert!(!r.roundtrip_passed());
    }

    #[test]
    fn write_report_id_is_independent_of_elapsed() {
        // INVARIANT-007: identical inputs, different wall-clock → identical id.
        let mk = |ms: u64| {
            KcubeWriteReport::new(
                d(b"pkg"),
                d(b"ep"),
                KcubeWriteOutcome::SkippedByReportOnly,
                None,
                0,
                None,
                vec![],
                bundle_id(),
                ms,
            )
        };
        assert_eq!(
            mk(100).id,
            mk(200).id,
            "report id must not depend on elapsed_ms"
        );
    }

    #[test]
    fn write_report_new_deterministic() {
        let r1 = KcubeWriteReport::new(
            d(b"pkg"),
            d(b"ep"),
            KcubeWriteOutcome::Written,
            Some(d(b"path")),
            1_024,
            None,
            vec![],
            bundle_id(),
            100,
        );
        let r2 = KcubeWriteReport::new(
            d(b"pkg"),
            d(b"ep"),
            KcubeWriteOutcome::Written,
            Some(d(b"path")),
            1_024,
            None,
            vec![],
            bundle_id(),
            100,
        );
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn write_report_evidence_mandatory() {
        let r = KcubeWriteReport::skipped_by_report_only(d(b"pkg"), d(b"ep"), bundle_id());
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn write_report_no_floats_in_written_bytes() {
        let r = KcubeWriteReport::new(
            d(b"pkg"),
            d(b"ep"),
            KcubeWriteOutcome::Written,
            None,
            8_192u64,
            None,
            vec![],
            bundle_id(),
            50,
        );
        assert_eq!(r.written_bytes, 8_192u64);
        assert_eq!(r.elapsed_ms, 50u64);
    }

    #[test]
    fn r6_report_only_mode_skips_write() {
        use crate::policy::{ImplementationMode, PolicyProfile};
        let profile = PolicyProfile::default();
        assert_eq!(profile.mode, ImplementationMode::ReportOnly);

        let export_policy = report_only_export_policy();
        assert!(!export_policy.allow_write);

        let r = KcubeWriteReport::skipped_by_report_only(d(b"pkg"), export_policy.id, bundle_id());
        assert!(r.outcome.is_skipped_report_only());
    }

    #[test]
    fn r6_roundtrip_required_by_default() {
        let p = report_only_export_policy();
        assert!(p.require_roundtrip_verification);
        let p2 = KcubeExportPolicy::write_once(policy_id(), target_dir(), vec![]);
        assert!(p2.require_roundtrip_verification);
    }

    #[test]
    fn r6_evidence_bundle_integration() {
        let ev_bundle = EvidenceBundle::seal(
            vec![EvidenceRef::new(
                d(b"export-ev"),
                EvidenceKind::AssimilationCertificate,
                "export evidence",
            )],
            policy_id(),
            ReplayStatus::Replayable,
        );
        let r = KcubeWriteReport::skipped_by_report_only(d(b"pkg"), d(b"ep"), ev_bundle.bundle_id);
        assert!(r.verify_id());
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn r6_package_digest_separate_from_descriptor_id() {
        // package_digest covers archive bytes; id covers the descriptor fields.
        let p = basic_package();
        assert_ne!(p.id, p.package_digest);
    }
}

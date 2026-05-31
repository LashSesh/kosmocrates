use crate::digest::Digest;
use serde::{Deserialize, Serialize};

/// Classifies the kind of source being acquired.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcquisitionSourceKind {
    /// Local filesystem path — the only kind permitted in default policy.
    LocalPath,
    /// Git repository (requires explicit network capability grant).
    GitRepository,
    /// Local archive file (`.tar.gz`, `.zip`, etc.).
    Archive,
    Custom(String),
}

/// Taint classification for an `AcquiredSource`.
///
/// Every acquired source starts as `Unverified`. It must progress through
/// license check and secret scan before reaching `Cleared`. Only `Cleared`
/// sources may proceed to a Foundry gate — they still may not enter
/// `ContextPack`, host patches, or trusted memory without operator approval.
///
/// SAFETY_POLICY §1: no raw external source code enters default prompts,
/// host patches, trusted memory, or ContextPack.
/// SAFETY_POLICY §2: no acquired repository is executed by default.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AcquisitionTaint {
    /// Initial state — no checks performed.
    Unverified,
    /// License has not been verified.
    LicenseNotChecked,
    /// Secret scan has not been run.
    SecretScanNotRun,
    /// Both license check and secret scan passed.
    /// Source may proceed to Foundry gate, but still requires
    /// explicit operator approval before any trusted use.
    Cleared,
}

impl AcquisitionTaint {
    pub fn is_cleared(&self) -> bool {
        matches!(self, Self::Cleared)
    }

    pub fn is_tainted(&self) -> bool {
        !self.is_cleared()
    }
}

/// Outcome of a license compliance check on an acquired source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenseCheckOutcome {
    /// License identified and approved.
    Passed { spdx_id: String },
    /// License check failed or license is incompatible.
    Failed { reason: String },
    /// License could not be determined.
    Inconclusive,
    /// Check was not yet run.
    NotRun,
}

impl LicenseCheckOutcome {
    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }
}

/// Outcome of a secret / credential scan on an acquired source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretScanOutcome {
    /// No secrets or credentials detected.
    Passed,
    /// One or more secrets or credentials detected.
    Failed { findings: Vec<String> },
    /// Scan could not complete.
    Inconclusive,
    /// Scan was not yet run.
    NotRun,
}

impl SecretScanOutcome {
    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed)
    }
}

/// Declares what acquisition operations the policy permits.
///
/// `allow_network` defaults to `false` — network acquisition is disabled
/// by the safety policy (`PolicyProfile.allow_network = false`).
/// `allow_local_path` is the only capability enabled in the default policy.
/// `require_license_check` and `require_secret_scan` default to `true`.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAcquisitionCapability {
    pub id: Digest,
    pub policy_id: Digest,
    pub allow_local_path: bool,
    pub allow_network: bool,
    pub allow_git_repository: bool,
    pub allowed_source_kinds: Vec<AcquisitionSourceKind>,
    pub require_license_check: bool,
    pub require_secret_scan: bool,
}

#[derive(Serialize)]
struct CapabilityContent<'a> {
    policy_id: &'a Digest,
    allow_local_path: bool,
    allow_network: bool,
    allow_git_repository: bool,
    allowed_source_kinds: &'a Vec<AcquisitionSourceKind>,
    require_license_check: bool,
    require_secret_scan: bool,
}

impl SourceAcquisitionCapability {
    /// Fail-closed default: local path only, network disabled, scans required.
    pub fn local_only(policy_id: Digest) -> Self {
        let mut c = Self {
            id: Digest::ZERO,
            policy_id,
            allow_local_path: true,
            allow_network: false,
            allow_git_repository: false,
            allowed_source_kinds: vec![AcquisitionSourceKind::LocalPath],
            require_license_check: true,
            require_secret_scan: true,
        };
        c.id = c.compute_id();
        c
    }

    /// Fully locked: no acquisition of any kind permitted.
    pub fn deny_all(policy_id: Digest) -> Self {
        let mut c = Self {
            id: Digest::ZERO,
            policy_id,
            allow_local_path: false,
            allow_network: false,
            allow_git_repository: false,
            allowed_source_kinds: vec![],
            require_license_check: true,
            require_secret_scan: true,
        };
        c.id = c.compute_id();
        c
    }

    pub fn is_source_kind_allowed(&self, kind: &AcquisitionSourceKind) -> bool {
        self.allowed_source_kinds.contains(kind)
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&CapabilityContent {
            policy_id: &self.policy_id,
            allow_local_path: self.allow_local_path,
            allow_network: self.allow_network,
            allow_git_repository: self.allow_git_repository,
            allowed_source_kinds: &self.allowed_source_kinds,
            require_license_check: self.require_license_check,
            require_secret_scan: self.require_secret_scan,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Isolation sandbox for acquired sources.
///
/// `allow_execution` is hardwired to `false` — no acquired repository is
/// executed by default (SAFETY_POLICY §2).
/// `allow_host_write` is hardwired to `false`.
/// `allow_context_injection` is hardwired to `false` — no raw external
/// source code enters ContextPack or prompts (SAFETY_POLICY §1).
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionSandbox {
    pub id: Digest,
    /// Content digest of the sandbox directory path.
    pub sandbox_path_digest: Digest,
    /// Always `false` — invariant enforced by constructor.
    pub allow_execution: bool,
    /// Always `false` — invariant enforced by constructor.
    pub allow_host_write: bool,
    /// Always `false` — no source code injected into context (SAFETY_POLICY §1).
    pub allow_context_injection: bool,
    pub policy_id: Digest,
}

#[derive(Serialize)]
struct SandboxContent<'a> {
    sandbox_path_digest: &'a Digest,
    allow_execution: bool,
    allow_host_write: bool,
    allow_context_injection: bool,
    policy_id: &'a Digest,
}

impl AcquisitionSandbox {
    pub fn new(sandbox_path_digest: Digest, policy_id: Digest) -> Self {
        let mut s = Self {
            id: Digest::ZERO,
            sandbox_path_digest,
            allow_execution: false,
            allow_host_write: false,
            allow_context_injection: false,
            policy_id,
        };
        s.id = s.compute_id();
        s
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&SandboxContent {
            sandbox_path_digest: &self.sandbox_path_digest,
            allow_execution: self.allow_execution,
            allow_host_write: self.allow_host_write,
            allow_context_injection: self.allow_context_injection,
            policy_id: &self.policy_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Content-addressed record of an acquired external source.
///
/// Taint starts as `Unverified`. It advances only after both
/// `license_check` passes and `secret_scan` passes.
/// Only a `Cleared` source may proceed to Foundry gate.
/// Even a `Cleared` source still requires explicit operator approval
/// before entering ContextPack, host patches, or trusted memory.
///
/// `size_bytes` is `u64` — no floats.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquiredSource {
    pub id: Digest,
    pub source_kind: AcquisitionSourceKind,
    /// Content digest of the source path reference.
    pub source_path_digest: Digest,
    /// Content digest of the actual acquired content.
    pub content_digest: Digest,
    pub taint: AcquisitionTaint,
    pub license_check: LicenseCheckOutcome,
    pub secret_scan: SecretScanOutcome,
    pub sandbox_id: Digest,
    pub evidence_bundle_id: Digest,
    pub policy_id: Digest,
    pub size_bytes: u64,
}

#[derive(Serialize)]
struct AcquiredSourceContent<'a> {
    source_kind: &'a AcquisitionSourceKind,
    source_path_digest: &'a Digest,
    content_digest: &'a Digest,
    taint: &'a AcquisitionTaint,
    license_check: &'a LicenseCheckOutcome,
    secret_scan: &'a SecretScanOutcome,
    sandbox_id: &'a Digest,
    evidence_bundle_id: &'a Digest,
    policy_id: &'a Digest,
    size_bytes: u64,
}

impl AcquiredSource {
    /// Construct a freshly acquired source — taint is always `Unverified`.
    pub fn new(
        source_kind: AcquisitionSourceKind,
        source_path_digest: Digest,
        content_digest: Digest,
        sandbox_id: Digest,
        evidence_bundle_id: Digest,
        policy_id: Digest,
        size_bytes: u64,
    ) -> Self {
        let mut s = Self {
            id: Digest::ZERO,
            source_kind,
            source_path_digest,
            content_digest,
            taint: AcquisitionTaint::Unverified,
            license_check: LicenseCheckOutcome::NotRun,
            secret_scan: SecretScanOutcome::NotRun,
            sandbox_id,
            evidence_bundle_id,
            policy_id,
            size_bytes,
        };
        s.id = s.compute_id();
        s
    }

    /// Apply the outcome of a license check, advancing taint if appropriate.
    pub fn with_license_check(mut self, outcome: LicenseCheckOutcome) -> Self {
        self.license_check = outcome;
        self.taint = Self::derive_taint(&self.license_check, &self.secret_scan);
        self.id = self.compute_id();
        self
    }

    /// Apply the outcome of a secret scan, advancing taint if appropriate.
    pub fn with_secret_scan(mut self, outcome: SecretScanOutcome) -> Self {
        self.secret_scan = outcome;
        self.taint = Self::derive_taint(&self.license_check, &self.secret_scan);
        self.id = self.compute_id();
        self
    }

    /// Derive taint from current check outcomes — called after each update.
    fn derive_taint(license: &LicenseCheckOutcome, secret: &SecretScanOutcome) -> AcquisitionTaint {
        match (license, secret) {
            (LicenseCheckOutcome::Passed { .. }, SecretScanOutcome::Passed) => {
                AcquisitionTaint::Cleared
            }
            (LicenseCheckOutcome::NotRun, _) => AcquisitionTaint::LicenseNotChecked,
            (_, SecretScanOutcome::NotRun) => AcquisitionTaint::SecretScanNotRun,
            _ => AcquisitionTaint::Unverified,
        }
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&AcquiredSourceContent {
            source_kind: &self.source_kind,
            source_path_digest: &self.source_path_digest,
            content_digest: &self.content_digest,
            taint: &self.taint,
            license_check: &self.license_check,
            secret_scan: &self.secret_scan,
            sandbox_id: &self.sandbox_id,
            evidence_bundle_id: &self.evidence_bundle_id,
            policy_id: &self.policy_id,
            size_bytes: self.size_bytes,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    /// Returns `true` iff the source has passed both checks and may proceed to Foundry gate.
    pub fn may_proceed_to_foundry(&self) -> bool {
        self.taint.is_cleared()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{EvidenceBundle, EvidenceKind, EvidenceRef, ReplayStatus};

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn policy_id() -> Digest { d(b"policy") }
    fn bundle_id() -> Digest { d(b"bundle") }

    fn basic_source() -> AcquiredSource {
        AcquiredSource::new(
            AcquisitionSourceKind::LocalPath,
            d(b"src-path"),
            d(b"content"),
            d(b"sandbox"),
            bundle_id(),
            policy_id(),
            4_096,
        )
    }

    // --- AcquisitionTaint ---

    #[test]
    fn taint_cleared_is_not_tainted() {
        assert!(AcquisitionTaint::Cleared.is_cleared());
        assert!(!AcquisitionTaint::Cleared.is_tainted());
    }

    #[test]
    fn taint_unverified_is_tainted() {
        assert!(AcquisitionTaint::Unverified.is_tainted());
        assert!(!AcquisitionTaint::Unverified.is_cleared());
    }

    #[test]
    fn taint_ordering_cleared_is_highest() {
        assert!(AcquisitionTaint::Cleared > AcquisitionTaint::Unverified);
        assert!(AcquisitionTaint::Cleared > AcquisitionTaint::LicenseNotChecked);
        assert!(AcquisitionTaint::Cleared > AcquisitionTaint::SecretScanNotRun);
    }

    // --- LicenseCheckOutcome / SecretScanOutcome ---

    #[test]
    fn license_passed_is_passed() {
        let o = LicenseCheckOutcome::Passed { spdx_id: "MIT".into() };
        assert!(o.is_passed());
    }

    #[test]
    fn license_failed_not_passed() {
        let o = LicenseCheckOutcome::Failed { reason: "GPL incompatible".into() };
        assert!(!o.is_passed());
    }

    #[test]
    fn secret_scan_passed_is_passed() {
        assert!(SecretScanOutcome::Passed.is_passed());
    }

    #[test]
    fn secret_scan_failed_not_passed() {
        let o = SecretScanOutcome::Failed { findings: vec!["AWS key".into()] };
        assert!(!o.is_passed());
    }

    // --- SourceAcquisitionCapability ---

    #[test]
    fn capability_local_only_id_deterministic() {
        let c1 = SourceAcquisitionCapability::local_only(policy_id());
        let c2 = SourceAcquisitionCapability::local_only(policy_id());
        assert_eq!(c1.id, c2.id);
    }

    #[test]
    fn capability_local_only_verify_id() {
        assert!(SourceAcquisitionCapability::local_only(policy_id()).verify_id());
    }

    #[test]
    fn capability_local_only_network_disabled() {
        let c = SourceAcquisitionCapability::local_only(policy_id());
        assert!(!c.allow_network);
        assert!(!c.allow_git_repository);
        assert!(c.allow_local_path);
        assert!(c.require_license_check);
        assert!(c.require_secret_scan);
    }

    #[test]
    fn capability_deny_all_blocks_everything() {
        let c = SourceAcquisitionCapability::deny_all(policy_id());
        assert!(!c.allow_local_path);
        assert!(!c.allow_network);
        assert!(!c.is_source_kind_allowed(&AcquisitionSourceKind::LocalPath));
        assert!(c.verify_id());
    }

    #[test]
    fn capability_is_source_kind_allowed() {
        let c = SourceAcquisitionCapability::local_only(policy_id());
        assert!(c.is_source_kind_allowed(&AcquisitionSourceKind::LocalPath));
        assert!(!c.is_source_kind_allowed(&AcquisitionSourceKind::GitRepository));
        assert!(!c.is_source_kind_allowed(&AcquisitionSourceKind::Archive));
    }

    #[test]
    fn capability_different_policies_differ() {
        let c1 = SourceAcquisitionCapability::local_only(d(b"pol-a"));
        let c2 = SourceAcquisitionCapability::local_only(d(b"pol-b"));
        assert_ne!(c1.id, c2.id);
    }

    // --- AcquisitionSandbox ---

    #[test]
    fn sandbox_id_deterministic() {
        let s1 = AcquisitionSandbox::new(d(b"path"), policy_id());
        let s2 = AcquisitionSandbox::new(d(b"path"), policy_id());
        assert_eq!(s1.id, s2.id);
    }

    #[test]
    fn sandbox_verify_id() {
        assert!(AcquisitionSandbox::new(d(b"path"), policy_id()).verify_id());
    }

    #[test]
    fn sandbox_all_allow_flags_false() {
        let s = AcquisitionSandbox::new(d(b"path"), policy_id());
        assert!(!s.allow_execution, "SAFETY_POLICY §2: no acquired repo executed");
        assert!(!s.allow_host_write);
        assert!(!s.allow_context_injection, "SAFETY_POLICY §1: no external code in context");
    }

    // --- AcquiredSource (initial state) ---

    #[test]
    fn source_id_deterministic() {
        let s1 = basic_source();
        let s2 = basic_source();
        assert_eq!(s1.id, s2.id);
    }

    #[test]
    fn source_verify_id() {
        assert!(basic_source().verify_id());
    }

    #[test]
    fn source_initial_taint_is_unverified() {
        let s = basic_source();
        assert_eq!(s.taint, AcquisitionTaint::Unverified);
        assert!(s.taint.is_tainted());
    }

    #[test]
    fn source_initial_checks_are_not_run() {
        let s = basic_source();
        assert_eq!(s.license_check, LicenseCheckOutcome::NotRun);
        assert_eq!(s.secret_scan, SecretScanOutcome::NotRun);
    }

    #[test]
    fn source_cannot_proceed_to_foundry_when_tainted() {
        assert!(!basic_source().may_proceed_to_foundry());
    }

    #[test]
    fn source_no_floats_in_size_bytes() {
        let s = basic_source();
        assert_eq!(s.size_bytes, 4_096u64);
    }

    // --- AcquiredSource taint progression ---

    #[test]
    fn source_license_check_alone_does_not_clear() {
        let s = basic_source()
            .with_license_check(LicenseCheckOutcome::Passed { spdx_id: "MIT".into() });
        assert_eq!(s.taint, AcquisitionTaint::SecretScanNotRun);
        assert!(s.taint.is_tainted());
        assert!(s.verify_id());
    }

    #[test]
    fn source_secret_scan_alone_does_not_clear() {
        let s = basic_source().with_secret_scan(SecretScanOutcome::Passed);
        assert_eq!(s.taint, AcquisitionTaint::LicenseNotChecked);
        assert!(s.taint.is_tainted());
        assert!(s.verify_id());
    }

    #[test]
    fn source_both_checks_passed_clears_taint() {
        let s = basic_source()
            .with_license_check(LicenseCheckOutcome::Passed { spdx_id: "Apache-2.0".into() })
            .with_secret_scan(SecretScanOutcome::Passed);
        assert_eq!(s.taint, AcquisitionTaint::Cleared);
        assert!(s.may_proceed_to_foundry());
        assert!(s.verify_id());
    }

    #[test]
    fn source_failed_license_does_not_clear() {
        let s = basic_source()
            .with_license_check(LicenseCheckOutcome::Failed { reason: "GPL".into() })
            .with_secret_scan(SecretScanOutcome::Passed);
        assert_eq!(s.taint, AcquisitionTaint::Unverified);
        assert!(!s.may_proceed_to_foundry());
        assert!(s.verify_id());
    }

    #[test]
    fn source_failed_secret_scan_does_not_clear() {
        let s = basic_source()
            .with_license_check(LicenseCheckOutcome::Passed { spdx_id: "MIT".into() })
            .with_secret_scan(SecretScanOutcome::Failed {
                findings: vec!["GITHUB_TOKEN".into()],
            });
        assert_eq!(s.taint, AcquisitionTaint::Unverified);
        assert!(!s.may_proceed_to_foundry());
        assert!(s.verify_id());
    }

    #[test]
    fn source_taint_changes_id() {
        let s1 = basic_source();
        let s2 = basic_source()
            .with_license_check(LicenseCheckOutcome::Passed { spdx_id: "MIT".into() });
        assert_ne!(s1.id, s2.id);
    }

    #[test]
    fn source_cleared_then_secret_scan_fail_reverts() {
        // Applying a failing scan after clearing must revert taint.
        let s = basic_source()
            .with_license_check(LicenseCheckOutcome::Passed { spdx_id: "MIT".into() })
            .with_secret_scan(SecretScanOutcome::Passed) // cleared
            .with_secret_scan(SecretScanOutcome::Failed {
                findings: vec!["AWS_SECRET".into()],
            });
        assert_eq!(s.taint, AcquisitionTaint::Unverified);
        assert!(!s.may_proceed_to_foundry());
        assert!(s.verify_id());
    }

    // --- Safety policy structural tests ---

    #[test]
    fn r8_network_disabled_by_default() {
        use crate::policy::PolicyProfile;
        let profile = PolicyProfile::default();
        assert!(!profile.allow_network);

        let cap = SourceAcquisitionCapability::local_only(policy_id());
        assert!(!cap.allow_network);
    }

    #[test]
    fn r8_no_execution_of_acquired_source() {
        let sandbox = AcquisitionSandbox::new(d(b"sandbox"), policy_id());
        assert!(!sandbox.allow_execution, "SAFETY_POLICY §2: never execute acquired repo");
    }

    #[test]
    fn r8_no_context_injection_from_external() {
        let sandbox = AcquisitionSandbox::new(d(b"sandbox"), policy_id());
        assert!(
            !sandbox.allow_context_injection,
            "SAFETY_POLICY §1: no raw external source in ContextPack"
        );
    }

    #[test]
    fn r8_tainted_source_blocked_from_foundry() {
        let s = basic_source(); // Unverified taint
        assert!(!s.may_proceed_to_foundry());
    }

    #[test]
    fn r8_cleared_source_allowed_for_foundry() {
        let s = basic_source()
            .with_license_check(LicenseCheckOutcome::Passed { spdx_id: "MIT".into() })
            .with_secret_scan(SecretScanOutcome::Passed);
        assert!(s.may_proceed_to_foundry());
        // Note: even Cleared requires explicit operator approval before trusted use.
    }

    #[test]
    fn r8_evidence_bundle_integration() {
        let ev_bundle = EvidenceBundle::seal(
            vec![EvidenceRef::new(
                d(b"acq-ev"),
                EvidenceKind::HostScan,
                "acquisition scan evidence",
            )],
            policy_id(),
            ReplayStatus::Replayable,
        );
        let s = AcquiredSource::new(
            AcquisitionSourceKind::LocalPath,
            d(b"path"),
            d(b"content"),
            d(b"sandbox"),
            ev_bundle.bundle_id,
            policy_id(),
            1_024,
        );
        assert!(s.verify_id());
        assert_ne!(s.evidence_bundle_id, Digest::ZERO);
    }
}

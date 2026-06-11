use crate::digest::Digest;
use crate::evidence::ReplayStatus;
use crate::policy::PolicyProfile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Scoping classification for a `CorpusCartographyStore`.
///
/// Determines what sources of evidence are eligible for inclusion.
/// CorpusCartography is NOT trusted memory — it is a read/plan-only
/// index regardless of scope (SAFETY_POLICY §Hard Safety Rules 4).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CorpusScope {
    /// Files within the current host project workspace only.
    LocalHostProject,
    /// All projects in the same local workspace family.
    WorkspaceFamily,
    /// Private organizational corpus (requires explicit policy elevation).
    OrganizationPrivateCorpus,
    /// Approved mirrors — explicit allow-list required.
    ApprovedMirrorCorpus,
    /// Public open-source corpus — highest scrutiny required.
    PublicOpenSourceCorpus,
    /// Named domain profile for custom scoping.
    DomainProfile(String),
}

/// Classifies the kind of artifact recorded in a `CartographyStoreCommit`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CartographyEntryKind {
    HostCubeSkeleton,
    TopologicalVoidMap,
    DeficiencyVector,
    SourceFrontierGraph,
    CorpusEntry,
    EvidenceSummary,
    /// Feedback from a PSE promotion outcome — carries knowledge back into the
    /// corpus (the "Wissen zurück ins Substrat" record).
    PromotionFeedback,
    /// A `CorpusCartographyUpdate` produced by a pipeline run, recording the
    /// delta between the corpus state before and after the run.
    CartographyUpdate,
    Custom(String),
}

/// A single append operation into a `CorpusCartographyStore`.
///
/// `id` is the content digest of all fields except `id`.
/// `sequence` provides total ordering within the store; must be
/// monotonically increasing and gapless within a policy scope.
/// `payload_digest` points to the actual artifact — the commit does
/// not inline the payload (content-addressing by reference).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartographyStoreCommit {
    pub id: Digest,
    pub scope: CorpusScope,
    pub sequence: u64,
    pub payload_digest: Digest,
    pub payload_kind: CartographyEntryKind,
    pub evidence_bundle_id: Digest,
    pub policy_id: Digest,
    pub labels: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct CommitContent<'a> {
    scope: &'a CorpusScope,
    sequence: u64,
    payload_digest: &'a Digest,
    payload_kind: &'a CartographyEntryKind,
    evidence_bundle_id: &'a Digest,
    policy_id: &'a Digest,
    labels: &'a BTreeMap<String, String>,
}

impl CartographyStoreCommit {
    pub fn new(
        scope: CorpusScope,
        sequence: u64,
        payload_digest: Digest,
        payload_kind: CartographyEntryKind,
        evidence_bundle_id: Digest,
        policy_id: Digest,
    ) -> Self {
        let mut c = Self {
            id: Digest::ZERO,
            scope,
            sequence,
            payload_digest,
            payload_kind,
            evidence_bundle_id,
            policy_id,
            labels: BTreeMap::new(),
        };
        c.id = c.compute_id();
        c
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self.id = self.compute_id();
        self
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&CommitContent {
            scope: &self.scope,
            sequence: self.sequence,
            payload_digest: &self.payload_digest,
            payload_kind: &self.payload_kind,
            evidence_bundle_id: &self.evidence_bundle_id,
            policy_id: &self.policy_id,
            labels: &self.labels,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// The append-only manifest of a `CorpusCartographyStore`.
///
/// `entries` is ordered by `sequence`; the manifest is immutable once sealed.
/// `id` is the content digest of all fields except `id`.
/// `head_sequence` is the sequence number of the last committed entry
/// (or 0 when the store is empty).
///
/// CorpusCartography is NOT trusted memory and must never be promoted
/// to trusted memory without explicit policy escalation (SAFETY_POLICY §4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartographyStorageManifest {
    pub id: Digest,
    pub scope: CorpusScope,
    pub entries: Vec<CartographyStoreCommit>,
    pub head_sequence: u64,
    pub policy_id: Digest,
    pub replay_status: ReplayStatus,
}

#[derive(Serialize)]
struct ManifestContent<'a> {
    scope: &'a CorpusScope,
    entry_ids: Vec<&'a Digest>,
    head_sequence: u64,
    policy_id: &'a Digest,
    replay_status: &'a ReplayStatus,
}

impl CartographyStorageManifest {
    pub fn empty(scope: CorpusScope, policy_id: Digest) -> Self {
        let mut m = Self {
            id: Digest::ZERO,
            scope,
            entries: vec![],
            head_sequence: 0,
            policy_id,
            replay_status: ReplayStatus::Replayable,
        };
        m.id = m.compute_id();
        m
    }

    /// Returns a new manifest with `commit` appended, recomputing `id`.
    ///
    /// Caller must ensure `commit.sequence == self.head_sequence + 1`
    /// (or `== 1` for the first entry). The store enforces this invariant.
    pub fn with_commit(mut self, commit: CartographyStoreCommit) -> Self {
        self.head_sequence = commit.sequence;
        self.entries.push(commit);
        self.id = self.compute_id();
        self
    }

    fn compute_id(&self) -> Digest {
        let entry_ids: Vec<&Digest> = self.entries.iter().map(|e| &e.id).collect();
        Digest::of(&ManifestContent {
            scope: &self.scope,
            entry_ids,
            head_sequence: self.head_sequence,
            policy_id: &self.policy_id,
            replay_status: &self.replay_status,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Outcome of a manifest integrity verification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CartographyIntegrityStatus {
    /// All entries have valid IDs and gapless sequences.
    Intact,
    /// Store is empty; no entries to verify.
    Empty,
    /// A sequence gap was detected.
    SequenceGap { expected: u64, found: u64 },
    /// A commit's stored `id` does not match its recomputed digest.
    DigestMismatch { commit_id: Digest },
}

impl CartographyIntegrityStatus {
    pub fn is_intact(&self) -> bool {
        matches!(self, Self::Intact | Self::Empty)
    }
}

/// Content-addressed result of a `CorpusCartographyStore` integrity check.
///
/// `evidence_bundle_id` is mandatory — every durable audit result is evidence-bound.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartographyIntegrityReport {
    pub id: Digest,
    pub manifest_id: Digest,
    pub status: CartographyIntegrityStatus,
    pub checked_count: u64,
    pub evidence_bundle_id: Digest,
}

#[derive(Serialize)]
struct IntegrityContent<'a> {
    manifest_id: &'a Digest,
    status: &'a CartographyIntegrityStatus,
    checked_count: u64,
    evidence_bundle_id: &'a Digest,
}

impl CartographyIntegrityReport {
    pub fn new(
        manifest_id: Digest,
        status: CartographyIntegrityStatus,
        checked_count: u64,
        evidence_bundle_id: Digest,
    ) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            manifest_id,
            status,
            checked_count,
            evidence_bundle_id,
        };
        r.id = r.compute_id();
        r
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&IntegrityContent {
            manifest_id: &self.manifest_id,
            status: &self.status,
            checked_count: self.checked_count,
            evidence_bundle_id: &self.evidence_bundle_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Error type for `CorpusCartographyStore` operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CartographyStoreError {
    /// Attempted to append a commit whose sequence is not `head + 1`.
    SequenceViolation { expected: u64, got: u64 },
    /// Policy forbids this operation (e.g., `ReportOnly` mode forbids mutation).
    PolicyDenied { reason: String },
    /// Scope mismatch between the store and the commit being appended.
    ScopeMismatch,
    /// Underlying I/O error (message only — no OS types in core).
    Io { message: String },
}

impl std::fmt::Display for CartographyStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SequenceViolation { expected, got } => {
                write!(f, "sequence violation: expected {expected}, got {got}")
            }
            Self::PolicyDenied { reason } => write!(f, "policy denied: {reason}"),
            Self::ScopeMismatch => write!(f, "scope mismatch"),
            Self::Io { message } => write!(f, "I/O error: {message}"),
        }
    }
}

impl std::error::Error for CartographyStoreError {}

/// Append-only, policy-scoped store interface for `CorpusCartography` artifacts.
///
/// Implementations must:
/// - Enforce monotonically increasing, gapless sequence numbers.
/// - Reject appends when `policy.mode == ReportOnly` (fail-closed).
/// - Never promote contents to trusted memory.
/// - Return a `CartographyIntegrityReport` from `verify_integrity()`.
pub trait CorpusCartographyStore {
    fn append(
        &mut self,
        commit: CartographyStoreCommit,
        policy: &PolicyProfile,
    ) -> Result<Digest, CartographyStoreError>;

    fn read_manifest(&self) -> Result<CartographyStorageManifest, CartographyStoreError>;

    fn verify_integrity(
        &self,
        evidence_bundle_id: Digest,
    ) -> Result<CartographyIntegrityReport, CartographyStoreError>;

    fn scope(&self) -> &CorpusScope;
}

/// In-memory implementation of `CorpusCartographyStore` for testing.
///
/// Does not write to disk. Enforces sequence monotonicity and policy checks.
/// Not for production use — the on-disk JSONL store is a separate implementation.
pub struct InMemoryCartographyStore {
    manifest: CartographyStorageManifest,
}

impl InMemoryCartographyStore {
    pub fn new(scope: CorpusScope, policy_id: Digest) -> Self {
        Self {
            manifest: CartographyStorageManifest::empty(scope, policy_id),
        }
    }
}

impl CorpusCartographyStore for InMemoryCartographyStore {
    fn append(
        &mut self,
        commit: CartographyStoreCommit,
        policy: &PolicyProfile,
    ) -> Result<Digest, CartographyStoreError> {
        use crate::policy::ImplementationMode;

        if policy.mode == ImplementationMode::ReportOnly {
            return Err(CartographyStoreError::PolicyDenied {
                reason: "ImplementationMode::ReportOnly forbids cartography store mutation".into(),
            });
        }

        if commit.scope != *self.manifest.scope() {
            return Err(CartographyStoreError::ScopeMismatch);
        }

        let expected_seq = self.manifest.head_sequence + 1;
        if commit.sequence != expected_seq {
            return Err(CartographyStoreError::SequenceViolation {
                expected: expected_seq,
                got: commit.sequence,
            });
        }

        let commit_id = commit.id;
        self.manifest = self.manifest.clone().with_commit(commit);
        Ok(commit_id)
    }

    fn read_manifest(&self) -> Result<CartographyStorageManifest, CartographyStoreError> {
        Ok(self.manifest.clone())
    }

    fn verify_integrity(
        &self,
        evidence_bundle_id: Digest,
    ) -> Result<CartographyIntegrityReport, CartographyStoreError> {
        if self.manifest.entries.is_empty() {
            return Ok(CartographyIntegrityReport::new(
                self.manifest.id,
                CartographyIntegrityStatus::Empty,
                0,
                evidence_bundle_id,
            ));
        }

        for (expected_seq, entry) in (1_u64..).zip(self.manifest.entries.iter()) {
            if !entry.verify_id() {
                return Ok(CartographyIntegrityReport::new(
                    self.manifest.id,
                    CartographyIntegrityStatus::DigestMismatch {
                        commit_id: entry.id,
                    },
                    expected_seq - 1,
                    evidence_bundle_id,
                ));
            }
            if entry.sequence != expected_seq {
                return Ok(CartographyIntegrityReport::new(
                    self.manifest.id,
                    CartographyIntegrityStatus::SequenceGap {
                        expected: expected_seq,
                        found: entry.sequence,
                    },
                    expected_seq - 1,
                    evidence_bundle_id,
                ));
            }
        }

        Ok(CartographyIntegrityReport::new(
            self.manifest.id,
            CartographyIntegrityStatus::Intact,
            self.manifest.entries.len() as u64,
            evidence_bundle_id,
        ))
    }

    fn scope(&self) -> &CorpusScope {
        &self.manifest.scope
    }
}

impl CartographyStorageManifest {
    fn scope(&self) -> &CorpusScope {
        &self.scope
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{EvidenceBundle, EvidenceKind, EvidenceRef, ReplayStatus};
    use crate::policy::{ImplementationMode, PolicyProfile};

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn policy_id() -> Digest {
        d(b"policy")
    }
    fn bundle_id() -> Digest {
        d(b"bundle")
    }
    fn payload_digest() -> Digest {
        d(b"payload")
    }

    fn report_only_policy() -> PolicyProfile {
        PolicyProfile::default()
    }

    fn elevated_policy() -> PolicyProfile {
        PolicyProfile {
            mode: ImplementationMode::DryRun,
            ..PolicyProfile::default()
        }
    }

    fn make_commit(sequence: u64, scope: CorpusScope) -> CartographyStoreCommit {
        CartographyStoreCommit::new(
            scope,
            sequence,
            payload_digest(),
            CartographyEntryKind::HostCubeSkeleton,
            bundle_id(),
            policy_id(),
        )
    }

    // --- CartographyStoreCommit ---

    #[test]
    fn commit_id_deterministic() {
        let c1 = make_commit(1, CorpusScope::LocalHostProject);
        let c2 = make_commit(1, CorpusScope::LocalHostProject);
        assert_eq!(c1.id, c2.id);
    }

    #[test]
    fn commit_verify_id() {
        let c = make_commit(1, CorpusScope::LocalHostProject);
        assert!(c.verify_id());
    }

    #[test]
    fn commit_different_sequence_differs() {
        let c1 = make_commit(1, CorpusScope::LocalHostProject);
        let c2 = make_commit(2, CorpusScope::LocalHostProject);
        assert_ne!(c1.id, c2.id);
    }

    #[test]
    fn commit_with_label_changes_id() {
        let c = make_commit(1, CorpusScope::LocalHostProject);
        let id_before = c.id;
        let c2 = c.with_label("phase", "R4");
        assert_ne!(id_before, c2.id);
        assert!(c2.verify_id());
    }

    #[test]
    fn commit_different_scopes_differ() {
        let c1 = make_commit(1, CorpusScope::LocalHostProject);
        let c2 = make_commit(1, CorpusScope::WorkspaceFamily);
        assert_ne!(c1.id, c2.id);
    }

    // --- CartographyStorageManifest ---

    #[test]
    fn manifest_empty_id_deterministic() {
        let m1 = CartographyStorageManifest::empty(CorpusScope::LocalHostProject, policy_id());
        let m2 = CartographyStorageManifest::empty(CorpusScope::LocalHostProject, policy_id());
        assert_eq!(m1.id, m2.id);
    }

    #[test]
    fn manifest_empty_verify_id() {
        let m = CartographyStorageManifest::empty(CorpusScope::LocalHostProject, policy_id());
        assert!(m.verify_id());
        assert!(m.is_empty());
        assert_eq!(m.head_sequence, 0);
    }

    #[test]
    fn manifest_with_commit_changes_id() {
        let m = CartographyStorageManifest::empty(CorpusScope::LocalHostProject, policy_id());
        let id_before = m.id;
        let commit = make_commit(1, CorpusScope::LocalHostProject);
        let m2 = m.with_commit(commit);
        assert_ne!(id_before, m2.id);
        assert!(m2.verify_id());
        assert_eq!(m2.head_sequence, 1);
        assert_eq!(m2.len(), 1);
    }

    #[test]
    fn manifest_multi_commit_deterministic() {
        let m = CartographyStorageManifest::empty(CorpusScope::LocalHostProject, policy_id());
        let c1 = make_commit(1, CorpusScope::LocalHostProject);
        let c2 = make_commit(2, CorpusScope::LocalHostProject);
        let m_final = m.with_commit(c1).with_commit(c2);
        assert_eq!(m_final.head_sequence, 2);
        assert_eq!(m_final.len(), 2);
        assert!(m_final.verify_id());
    }

    // --- CartographyIntegrityStatus ---

    #[test]
    fn integrity_status_intact_is_intact() {
        assert!(CartographyIntegrityStatus::Intact.is_intact());
        assert!(CartographyIntegrityStatus::Empty.is_intact());
    }

    #[test]
    fn integrity_status_failures_not_intact() {
        assert!(!CartographyIntegrityStatus::SequenceGap {
            expected: 2,
            found: 3
        }
        .is_intact());
        assert!(!CartographyIntegrityStatus::DigestMismatch {
            commit_id: Digest::ZERO
        }
        .is_intact());
    }

    // --- CartographyIntegrityReport ---

    #[test]
    fn integrity_report_id_deterministic() {
        let r1 = CartographyIntegrityReport::new(
            d(b"manifest"),
            CartographyIntegrityStatus::Intact,
            5,
            bundle_id(),
        );
        let r2 = CartographyIntegrityReport::new(
            d(b"manifest"),
            CartographyIntegrityStatus::Intact,
            5,
            bundle_id(),
        );
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn integrity_report_verify_id() {
        let r = CartographyIntegrityReport::new(
            d(b"manifest"),
            CartographyIntegrityStatus::Empty,
            0,
            bundle_id(),
        );
        assert!(r.verify_id());
    }

    #[test]
    fn integrity_report_evidence_mandatory() {
        let r = CartographyIntegrityReport::new(
            d(b"manifest"),
            CartographyIntegrityStatus::Intact,
            3,
            bundle_id(),
        );
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
    }

    // --- InMemoryCartographyStore ---

    #[test]
    fn store_append_report_only_denied() {
        let mut store = InMemoryCartographyStore::new(CorpusScope::LocalHostProject, policy_id());
        let commit = make_commit(1, CorpusScope::LocalHostProject);
        let result = store.append(commit, &report_only_policy());
        assert!(matches!(
            result,
            Err(CartographyStoreError::PolicyDenied { .. })
        ));
    }

    #[test]
    fn store_append_elevated_policy_succeeds() {
        let mut store = InMemoryCartographyStore::new(CorpusScope::LocalHostProject, policy_id());
        let commit = make_commit(1, CorpusScope::LocalHostProject);
        let result = store.append(commit, &elevated_policy());
        assert!(result.is_ok());
    }

    #[test]
    fn store_append_sequence_violation() {
        let mut store = InMemoryCartographyStore::new(CorpusScope::LocalHostProject, policy_id());
        let policy = elevated_policy();
        let c1 = make_commit(1, CorpusScope::LocalHostProject);
        store.append(c1, &policy).unwrap();
        let c3 = make_commit(3, CorpusScope::LocalHostProject); // gap: should be 2
        let err = store.append(c3, &policy).unwrap_err();
        assert!(matches!(
            err,
            CartographyStoreError::SequenceViolation {
                expected: 2,
                got: 3
            }
        ));
    }

    #[test]
    fn store_append_scope_mismatch() {
        let mut store = InMemoryCartographyStore::new(CorpusScope::LocalHostProject, policy_id());
        let commit = make_commit(1, CorpusScope::WorkspaceFamily);
        let err = store.append(commit, &elevated_policy()).unwrap_err();
        assert!(matches!(err, CartographyStoreError::ScopeMismatch));
    }

    #[test]
    fn store_read_manifest_after_appends() {
        let mut store = InMemoryCartographyStore::new(CorpusScope::LocalHostProject, policy_id());
        let policy = elevated_policy();
        store
            .append(make_commit(1, CorpusScope::LocalHostProject), &policy)
            .unwrap();
        store
            .append(make_commit(2, CorpusScope::LocalHostProject), &policy)
            .unwrap();
        let manifest = store.read_manifest().unwrap();
        assert_eq!(manifest.len(), 2);
        assert_eq!(manifest.head_sequence, 2);
        assert!(manifest.verify_id());
    }

    #[test]
    fn store_verify_integrity_empty() {
        let store = InMemoryCartographyStore::new(CorpusScope::LocalHostProject, policy_id());
        let report = store.verify_integrity(bundle_id()).unwrap();
        assert_eq!(report.status, CartographyIntegrityStatus::Empty);
        assert_eq!(report.checked_count, 0);
        assert!(report.verify_id());
    }

    #[test]
    fn store_verify_integrity_intact() {
        let mut store = InMemoryCartographyStore::new(CorpusScope::LocalHostProject, policy_id());
        let policy = elevated_policy();
        store
            .append(make_commit(1, CorpusScope::LocalHostProject), &policy)
            .unwrap();
        store
            .append(make_commit(2, CorpusScope::LocalHostProject), &policy)
            .unwrap();
        store
            .append(make_commit(3, CorpusScope::LocalHostProject), &policy)
            .unwrap();
        let report = store.verify_integrity(bundle_id()).unwrap();
        assert_eq!(report.status, CartographyIntegrityStatus::Intact);
        assert_eq!(report.checked_count, 3);
        assert!(report.verify_id());
    }

    #[test]
    fn store_scope_accessor() {
        let store = InMemoryCartographyStore::new(CorpusScope::WorkspaceFamily, policy_id());
        assert_eq!(store.scope(), &CorpusScope::WorkspaceFamily);
    }

    #[test]
    fn store_manifest_id_changes_on_each_append() {
        let mut store = InMemoryCartographyStore::new(CorpusScope::LocalHostProject, policy_id());
        let policy = elevated_policy();
        let id_0 = store.read_manifest().unwrap().id;
        store
            .append(make_commit(1, CorpusScope::LocalHostProject), &policy)
            .unwrap();
        let id_1 = store.read_manifest().unwrap().id;
        store
            .append(make_commit(2, CorpusScope::LocalHostProject), &policy)
            .unwrap();
        let id_2 = store.read_manifest().unwrap().id;
        assert_ne!(id_0, id_1);
        assert_ne!(id_1, id_2);
    }

    #[test]
    fn r4_corpus_cartography_not_trusted_memory() {
        // Structural test: ReportOnly mode must deny all append operations.
        // CorpusCartography is never trusted memory (SAFETY_POLICY §4).
        let mut store = InMemoryCartographyStore::new(CorpusScope::LocalHostProject, policy_id());
        for seq in 1..=5u64 {
            let commit = make_commit(seq, CorpusScope::LocalHostProject);
            let err = store.append(commit, &report_only_policy()).unwrap_err();
            assert!(
                matches!(err, CartographyStoreError::PolicyDenied { .. }),
                "ReportOnly must deny append at seq {seq}"
            );
        }
        // Store remains empty — no mutations occurred.
        let manifest = store.read_manifest().unwrap();
        assert!(manifest.is_empty());
    }

    #[test]
    fn r4_evidence_bundle_integration() {
        let ev_bundle = EvidenceBundle::seal(
            vec![EvidenceRef::new(
                d(b"scan-ev"),
                EvidenceKind::HostScan,
                "cartography scan evidence",
            )],
            policy_id(),
            ReplayStatus::Replayable,
        );
        let mut store = InMemoryCartographyStore::new(CorpusScope::LocalHostProject, policy_id());
        let policy = elevated_policy();
        store
            .append(make_commit(1, CorpusScope::LocalHostProject), &policy)
            .unwrap();
        let report = store.verify_integrity(ev_bundle.bundle_id).unwrap();
        assert_ne!(report.evidence_bundle_id, Digest::ZERO);
        assert!(report.verify_id());
    }

    #[test]
    fn r4_domain_profile_scope() {
        let scope = CorpusScope::DomainProfile("rust-ecosys".into());
        let store = InMemoryCartographyStore::new(scope.clone(), policy_id());
        assert_eq!(store.scope(), &scope);
    }
}

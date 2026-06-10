use kosmo_core::{AuthorityLabel, Digest, EvidenceKind, EvidenceRef, PolicyProfile, TaintLabel};
use serde::{Deserialize, Serialize};

/// Permitted use label for a ContextPack entry.
///
/// Every entry must declare what it may be used for.
/// An entry without permitted uses is inert — it exists but cannot
/// be referenced by any gate, task, or materialization path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PermittedUse {
    Analysis,
    Reporting,
    DigestComputation,
    GateCheck,
    FoundryInput,
}

/// Classifies the kind of artifact stored in a ContextPack entry.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ContextEntryKind {
    WorkspaceSnapshot,
    HostScanResult,
    DiagnosticReport,
    PolicyDocument,
    FoundryResult,
    TaskSpec,
}

/// A single entry in a ContextPack. Points to an artifact by digest.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextPackEntry {
    pub artifact_id: Digest,
    pub kind: ContextEntryKind,
    pub permitted_uses: Vec<PermittedUse>,
    pub taint: TaintLabel,
    pub authority: AuthorityLabel,
    pub label: String,
}

impl ContextPackEntry {
    pub fn new(
        artifact_id: Digest,
        kind: ContextEntryKind,
        taint: TaintLabel,
        authority: AuthorityLabel,
        label: impl Into<String>,
    ) -> Self {
        Self {
            artifact_id,
            kind,
            permitted_uses: vec![PermittedUse::Analysis, PermittedUse::Reporting],
            taint,
            authority,
            label: label.into(),
        }
    }
}

/// Serialize-only content struct for content-addressing.
#[derive(Serialize)]
struct PackContent<'a> {
    entries: &'a [ContextPackEntry],
    policy_id: &'a Digest,
    taint: &'a TaintLabel,
}

/// A policy-scoped, taint-propagating collection of artifact references.
///
/// No raw source code may enter a ContextPack by default (CROSS-005).
/// Any entry with `TaintLabel::External` is rejected unless the policy
/// explicitly enables `allow_context_injection_from_external`.
///
/// Taint propagates: adding a tainted entry raises the pack's overall taint.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextPack {
    pub pack_id: Digest,
    pub entries: Vec<ContextPackEntry>,
    pub policy_id: Digest,
    /// Highest taint level of any contained entry.
    pub taint: TaintLabel,
}

impl ContextPack {
    pub fn empty(policy_id: Digest) -> Self {
        let taint = TaintLabel::Clean;
        let id = Digest::of(&PackContent {
            entries: &[],
            policy_id: &policy_id,
            taint: &taint,
        });
        Self {
            pack_id: id,
            entries: vec![],
            policy_id,
            taint,
        }
    }

    /// Add an entry, enforcing CROSS-005 and taint propagation.
    pub fn add_entry(
        &mut self,
        entry: ContextPackEntry,
        policy: &PolicyProfile,
    ) -> Result<(), ContextPackError> {
        // CROSS-005: raw external content blocked unless policy explicitly allows
        if entry.taint == TaintLabel::External && !policy.allow_context_injection_from_external {
            return Err(ContextPackError::ExternalContentDenied);
        }
        if matches!(entry.taint, TaintLabel::Quarantined { .. }) {
            // Quarantined content is always blocked regardless of policy
            return Err(ContextPackError::QuarantinedContentBlocked);
        }
        // Propagate taint upward
        if entry.taint > self.taint {
            self.taint = entry.taint.clone();
        }
        self.entries.push(entry);
        self.pack_id = self.compute_id();
        Ok(())
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&PackContent {
            entries: &self.entries,
            policy_id: &self.policy_id,
            taint: &self.taint,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.pack_id == self.compute_id()
    }

    pub fn to_evidence_ref(&self) -> EvidenceRef {
        EvidenceRef::new(self.pack_id, EvidenceKind::RunRecord, "context-pack")
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn is_clean(&self) -> bool {
        self.taint == TaintLabel::Clean
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ContextPackError {
    #[error("external content denied: allow_context_injection_from_external=false (CROSS-005)")]
    ExternalContentDenied,
    #[error("quarantined content blocked unconditionally")]
    QuarantinedContentBlocked,
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{AuthorityLabel, Digest, PolicyProfile, TaintLabel};

    fn default_policy_id() -> Digest {
        PolicyProfile::default_report_only().id
    }

    #[test]
    fn context_pack_empty_is_clean() {
        let pack = ContextPack::empty(default_policy_id());
        assert!(pack.is_empty());
        assert!(pack.is_clean());
        assert!(pack.verify_id());
    }

    #[test]
    fn context_pack_clean_entry_accepted() {
        let mut pack = ContextPack::empty(default_policy_id());
        let entry = ContextPackEntry::new(
            Digest::of_bytes(b"scan"),
            ContextEntryKind::HostScanResult,
            TaintLabel::Clean,
            AuthorityLabel::Foundry,
            "workspace scan",
        );
        let policy = PolicyProfile::default_report_only();
        assert!(pack.add_entry(entry, &policy).is_ok());
        assert_eq!(pack.entries.len(), 1);
        assert!(pack.verify_id());
    }

    #[test]
    fn cross_005_external_tainted_entry_rejected_by_default() {
        // CROSS-005: raw external code never enters default ContextPack
        let mut pack = ContextPack::empty(default_policy_id());
        let entry = ContextPackEntry::new(
            Digest::of_bytes(b"external-code"),
            ContextEntryKind::WorkspaceSnapshot,
            TaintLabel::External,
            AuthorityLabel::Unknown,
            "external source",
        );
        let policy = PolicyProfile::default_report_only();
        let result = pack.add_entry(entry, &policy);
        assert!(
            matches!(result, Err(ContextPackError::ExternalContentDenied)),
            "expected ExternalContentDenied, got {result:?}"
        );
        assert!(pack.is_empty(), "pack must remain empty after rejection");
    }

    #[test]
    fn taint_propagates_to_pack() {
        let mut pack = ContextPack::empty(default_policy_id());
        // Use Synthetic taint (not External — External is blocked by default policy)
        let entry = ContextPackEntry::new(
            Digest::of_bytes(b"synth"),
            ContextEntryKind::DiagnosticReport,
            TaintLabel::Synthetic,
            AuthorityLabel::Agent {
                name: "hyphae".into(),
            },
            "synthetic output",
        );
        let policy = PolicyProfile::default_report_only();
        pack.add_entry(entry, &policy).unwrap();
        assert_eq!(pack.taint, TaintLabel::Synthetic);
    }

    #[test]
    fn context_pack_id_changes_after_add() {
        let pid = default_policy_id();
        let mut pack = ContextPack::empty(pid);
        let id_before = pack.pack_id;
        let entry = ContextPackEntry::new(
            Digest::of_bytes(b"x"),
            ContextEntryKind::FoundryResult,
            TaintLabel::Clean,
            AuthorityLabel::Foundry,
            "result",
        );
        pack.add_entry(entry, &PolicyProfile::default_report_only())
            .unwrap();
        assert_ne!(pack.pack_id, id_before);
        assert!(pack.verify_id());
    }
}

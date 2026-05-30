use crate::digest::Digest;
use serde::{Deserialize, Serialize};

/// Replay guarantee of an artifact or bundle.
///
/// Every durable artifact must carry a `ReplayStatus` (CROSS-006, CROSS-015).
/// Objects that cannot guarantee replay must be explicitly marked
/// `ReplayIncomplete` rather than silently omitting the status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayStatus {
    /// Artifact can be deterministically reproduced from its inputs.
    Replayable,
    /// Replay is possible in principle but not fully guaranteed; reason is recorded.
    ReplayIncomplete { reason: String },
    /// Replay is not applicable (e.g. ephemeral event record).
    NotApplicable,
}

impl Default for ReplayStatus {
    fn default() -> Self {
        ReplayStatus::ReplayIncomplete {
            reason: "not yet verified".into(),
        }
    }
}

/// Classifies what kind of evidence an `EvidenceRef` points to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EvidenceKind {
    Observation,
    GateTrace,
    RunRecord,
    FoundryCheck,
    HostScan,
    StructuralYield,
    AssimilationCertificate,
    External,
    Custom(String),
}

/// A content-addressed reference to a piece of evidence.
///
/// `digest` points to the actual evidence artifact. `kind` classifies the
/// type of evidence. `label` is a human-readable description.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub digest: Digest,
    pub kind: EvidenceKind,
    pub label: String,
}

impl EvidenceRef {
    pub fn new(digest: Digest, kind: EvidenceKind, label: impl Into<String>) -> Self {
        Self {
            digest,
            kind,
            label: label.into(),
        }
    }
}

/// Internal struct for content-addressing an `EvidenceBundle`.
/// Excludes `bundle_id` to avoid self-reference. Serialize-only.
#[derive(Serialize)]
struct EvidenceBundleContent<'a> {
    refs: &'a Vec<EvidenceRef>,
    policy_id: &'a Digest,
    replay_status: &'a ReplayStatus,
}

/// A content-addressed, policy-scoped collection of evidence references.
///
/// `bundle_id` is the canonical digest of `(refs, policy_id, replay_status)`.
/// Satisfies CROSS-006: every durable object has digest, evidence refs, policy scope,
/// and replay status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub bundle_id: Digest,
    pub refs: Vec<EvidenceRef>,
    pub policy_id: Digest,
    pub replay_status: ReplayStatus,
}

impl EvidenceBundle {
    /// Construct a sealed bundle: compute `bundle_id` from content.
    pub fn seal(
        refs: Vec<EvidenceRef>,
        policy_id: Digest,
        replay_status: ReplayStatus,
    ) -> Self {
        let id = Digest::of(&EvidenceBundleContent {
            refs: &refs,
            policy_id: &policy_id,
            replay_status: &replay_status,
        });
        Self {
            bundle_id: id,
            refs,
            policy_id,
            replay_status,
        }
    }

    /// Empty bundle scoped to a policy, marked not-applicable for replay.
    pub fn empty(policy_id: Digest) -> Self {
        Self::seal(Vec::new(), policy_id, ReplayStatus::NotApplicable)
    }

    /// Return a new bundle with `r` appended, recomputing `bundle_id`.
    pub fn with_ref(mut self, r: EvidenceRef) -> Self {
        self.refs.push(r);
        let id = Digest::of(&EvidenceBundleContent {
            refs: &self.refs,
            policy_id: &self.policy_id,
            replay_status: &self.replay_status,
        });
        self.bundle_id = id;
        self
    }

    /// Verify that `bundle_id` matches the current content.
    pub fn verify_id(&self) -> bool {
        let expected = Digest::of(&EvidenceBundleContent {
            refs: &self.refs,
            policy_id: &self.policy_id,
            replay_status: &self.replay_status,
        });
        self.bundle_id == expected
    }

    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_policy_id() -> Digest {
        Digest::of_bytes(b"test-policy")
    }

    #[test]
    fn evidence_bundle_seal_deterministic() {
        let refs = vec![EvidenceRef::new(
            Digest::of_bytes(b"obs1"),
            EvidenceKind::Observation,
            "first observation",
        )];
        let b1 = EvidenceBundle::seal(refs.clone(), dummy_policy_id(), ReplayStatus::Replayable);
        let b2 = EvidenceBundle::seal(refs, dummy_policy_id(), ReplayStatus::Replayable);
        assert_eq!(b1.bundle_id, b2.bundle_id);
    }

    #[test]
    fn evidence_bundle_verify_id_passes() {
        let b = EvidenceBundle::seal(
            vec![],
            dummy_policy_id(),
            ReplayStatus::NotApplicable,
        );
        assert!(b.verify_id());
    }

    #[test]
    fn evidence_bundle_with_ref_updates_id() {
        let b = EvidenceBundle::empty(dummy_policy_id());
        let id_before = b.bundle_id;
        let b2 = b.with_ref(EvidenceRef::new(
            Digest::of_bytes(b"x"),
            EvidenceKind::HostScan,
            "scan",
        ));
        assert_ne!(id_before, b2.bundle_id);
        assert!(b2.verify_id());
    }

    #[test]
    fn cross_006_bundle_has_digest_evidence_policy_replay() {
        // CROSS-006: Every durable object has digest, evidence refs, policy scope, replay status.
        let b = EvidenceBundle::seal(
            vec![EvidenceRef::new(
                Digest::of_bytes(b"e"),
                EvidenceKind::GateTrace,
                "gate",
            )],
            dummy_policy_id(),
            ReplayStatus::Replayable,
        );
        assert_ne!(b.bundle_id, Digest::ZERO);
        assert!(!b.refs.is_empty());
        assert_ne!(b.policy_id, Digest::ZERO);
        assert_eq!(b.replay_status, ReplayStatus::Replayable);
    }
}

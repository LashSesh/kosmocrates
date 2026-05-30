//! BlueprintUnit — the atomic topological export unit of a SystemCube.
//!
//! A `BlueprintUnit` is NOT raw source code. It is an evidence-bound,
//! content-addressed topological descriptor that can be included in a
//! `SystemCubeManifest`. Opaque (non-evidence-bound) units are rejected.

use kosmo_core::{AuthorityLabel, Digest, PolicyProfile, TaintLabel};
use serde::{Deserialize, Serialize};

// ─── Blueprint Unit Kind ──────────────────────────────────────────────────────

/// The structural kind of a `BlueprintUnit`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlueprintUnitKind {
    /// A topology descriptor for a module boundary.
    ModuleBoundary,
    /// A topology descriptor for a fiber / dependency edge.
    FiberDescriptor,
    /// A motif descriptor derived from a `MotifCandidate`.
    MotifDescriptor,
    /// A structural crystal reference (not executable code).
    CrystalReference,
    /// A norm gene descriptor (not trusted norm).
    NormDescriptor,
    /// Custom extension.
    Custom(String),
}

// ─── Blueprint Unit Status ────────────────────────────────────────────────────

/// Acceptance status for a `BlueprintUnit`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlueprintUnitStatus {
    /// Unit is evidence-bound and accepted into the manifest.
    Accepted,
    /// Unit is rejected because it lacks evidence binding.
    RejectedOpaque,
    /// Unit is accepted but carries a taint that limits trust.
    AcceptedWithTaint,
}

// ─── Blueprint Unit ───────────────────────────────────────────────────────────

/// Content for deterministic `BlueprintUnit` ID.
#[derive(Serialize)]
struct BlueprintUnitContent {
    kind: String,
    source_ref: Digest,
    authority: String,
    taint: String,
    policy_id: Digest,
    evidence_count: u32,
}

/// An atomic, evidence-bound topological export unit.
///
/// Blueprint units are topological descriptors, not raw source code.
/// Opaque units (missing evidence) are rejected at manifest assembly time.
/// The `status` field records the acceptance decision.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlueprintUnit {
    pub unit_id: Digest,
    pub kind: BlueprintUnitKind,
    /// Content-addressed reference to the topological source (e.g. StructuralCrystalRecord).
    pub source_ref: Digest,
    pub authority: AuthorityLabel,
    pub taint: TaintLabel,
    pub policy_id: Digest,
    /// Evidence refs that justify this unit's inclusion.
    pub evidence_ids: Vec<Digest>,
    pub status: BlueprintUnitStatus,
}

impl BlueprintUnit {
    pub fn new(
        kind: BlueprintUnitKind,
        source_ref: Digest,
        authority: AuthorityLabel,
        taint: TaintLabel,
        mut evidence_ids: Vec<Digest>,
        policy: &PolicyProfile,
    ) -> Self {
        evidence_ids.sort();
        let status = if evidence_ids.is_empty() {
            BlueprintUnitStatus::RejectedOpaque
        } else if taint != TaintLabel::Clean {
            BlueprintUnitStatus::AcceptedWithTaint
        } else {
            BlueprintUnitStatus::Accepted
        };
        let unit_id = Digest::of(&BlueprintUnitContent {
            kind: format!("{:?}", kind),
            source_ref,
            authority: format!("{:?}", authority),
            taint: format!("{:?}", taint),
            policy_id: policy.id,
            evidence_count: evidence_ids.len() as u32,
        });
        Self {
            unit_id,
            kind,
            source_ref,
            authority,
            taint,
            policy_id: policy.id,
            evidence_ids,
            status,
        }
    }

    /// Whether this unit is accepted into a manifest (not opaque-rejected).
    pub fn is_accepted(&self) -> bool {
        !matches!(self.status, BlueprintUnitStatus::RejectedOpaque)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{AuthorityLabel, PolicyProfile, TaintLabel};

    fn policy() -> PolicyProfile {
        PolicyProfile::default_report_only()
    }

    fn src() -> Digest {
        Digest::of_bytes(b"src")
    }

    fn ev() -> Digest {
        Digest::of_bytes(b"ev")
    }

    #[test]
    fn blueprint_unit_accepted_with_evidence() {
        let u = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            src(),
            AuthorityLabel::Operator,
            TaintLabel::Clean,
            vec![ev()],
            &policy(),
        );
        assert_eq!(u.status, BlueprintUnitStatus::Accepted);
        assert!(u.is_accepted());
    }

    #[test]
    fn blueprint_unit_rejected_opaque_without_evidence() {
        let u = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            src(),
            AuthorityLabel::Operator,
            TaintLabel::Clean,
            vec![],
            &policy(),
        );
        assert_eq!(u.status, BlueprintUnitStatus::RejectedOpaque);
        assert!(!u.is_accepted());
    }

    #[test]
    fn blueprint_unit_tainted_is_accepted_with_taint() {
        let u = BlueprintUnit::new(
            BlueprintUnitKind::MotifDescriptor,
            src(),
            AuthorityLabel::Operator,
            TaintLabel::Synthetic,
            vec![ev()],
            &policy(),
        );
        assert_eq!(u.status, BlueprintUnitStatus::AcceptedWithTaint);
        assert!(u.is_accepted());
    }

    #[test]
    fn blueprint_unit_is_content_addressed() {
        let u1 = BlueprintUnit::new(
            BlueprintUnitKind::FiberDescriptor,
            src(),
            AuthorityLabel::Operator,
            TaintLabel::Clean,
            vec![ev()],
            &policy(),
        );
        let u2 = BlueprintUnit::new(
            BlueprintUnitKind::FiberDescriptor,
            src(),
            AuthorityLabel::Operator,
            TaintLabel::Clean,
            vec![ev()],
            &policy(),
        );
        assert_eq!(u1.unit_id, u2.unit_id);
        assert_ne!(u1.unit_id, Digest::ZERO);
    }

    #[test]
    fn blueprint_unit_evidence_ids_sorted() {
        let e1 = Digest::of_bytes(b"e1");
        let e2 = Digest::of_bytes(b"e2");
        let u = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            src(),
            AuthorityLabel::Operator,
            TaintLabel::Clean,
            vec![e2, e1],
            &policy(),
        );
        assert_eq!(u.evidence_ids, vec![e1.min(e2), e1.max(e2)]);
    }
}

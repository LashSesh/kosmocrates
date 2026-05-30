//! Compatibility profile report for a SystemCube.
//!
//! A `CompatibilityProfileReport` describes how well the blueprint units of a
//! `SystemCubeManifest` align with the host's current topological profile.
//! The report is advisory only — a high compatibility score does not authorise
//! materialization.

use kosmo_core::{Digest, PolicyProfile, Q16};
use serde::{Deserialize, Serialize};

// ─── Compatibility Gap ─────────────────────────────────────────────────────────

/// A structural gap between a blueprint unit and the host topology.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompatibilityGap {
    pub unit_id: Digest,
    pub gap_kind: String,
    /// Q16-scaled severity of the gap.
    pub severity: Q16,
}

// ─── Compatibility Status ──────────────────────────────────────────────────────

/// Whether a compatibility profile could be computed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityStatus {
    /// Profile computed successfully.
    Available,
    /// No host snapshot available for comparison.
    NoHostSnapshot,
    /// Manifest is empty — nothing to compare.
    EmptyManifest,
}

// ─── Compatibility Profile Report ─────────────────────────────────────────────

/// Content for deterministic `CompatibilityProfileReport` ID.
#[derive(Serialize)]
struct CompatibilityContent {
    manifest_id: Digest,
    host_snapshot_id: Digest,
    policy_id: Digest,
    status: String,
    score_raw: i64,
    gap_count: u32,
}

/// Advisory compatibility profile between a `SystemCubeManifest` and the host.
///
/// `compatibility_score` is a Q16 value in [0, ONE]: ONE = perfect alignment.
/// Gaps are structural mismatches that a Foundry check must address before
/// any materialization is attempted.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompatibilityProfileReport {
    pub report_id: Digest,
    pub manifest_id: Digest,
    pub host_snapshot_id: Digest,
    pub policy_id: Digest,
    pub status: CompatibilityStatus,
    /// Q16 compatibility score in [0, ONE]. Advisory only.
    pub compatibility_score: Q16,
    /// Structural gaps sorted by unit_id for determinism.
    pub gaps: Vec<CompatibilityGap>,
}

impl CompatibilityProfileReport {
    pub fn new(
        manifest_id: Digest,
        host_snapshot_id: Digest,
        policy: &PolicyProfile,
        status: CompatibilityStatus,
        compatibility_score: Q16,
        mut gaps: Vec<CompatibilityGap>,
    ) -> Self {
        gaps.sort_by_key(|g| g.unit_id);
        let report_id = Digest::of(&CompatibilityContent {
            manifest_id,
            host_snapshot_id,
            policy_id: policy.id,
            status: format!("{:?}", status),
            score_raw: compatibility_score.raw(),
            gap_count: gaps.len() as u32,
        });
        Self {
            report_id,
            manifest_id,
            host_snapshot_id,
            policy_id: policy.id,
            status,
            compatibility_score,
            gaps,
        }
    }

    /// Build a report when no host snapshot is available for comparison.
    pub fn no_host_snapshot(
        manifest_id: Digest,
        policy: &PolicyProfile,
    ) -> Self {
        Self::new(
            manifest_id,
            Digest::ZERO,
            policy,
            CompatibilityStatus::NoHostSnapshot,
            Q16::ZERO,
            vec![],
        )
    }

    /// Build a fully compatible report (no gaps, score = ONE).
    pub fn perfect(
        manifest_id: Digest,
        host_snapshot_id: Digest,
        policy: &PolicyProfile,
    ) -> Self {
        Self::new(
            manifest_id,
            host_snapshot_id,
            policy,
            CompatibilityStatus::Available,
            Q16::ONE,
            vec![],
        )
    }

    pub fn summary(&self) -> String {
        format!(
            "CompatibilityProfileReport — status={:?} score={} gaps={}",
            self.status,
            self.compatibility_score.raw(),
            self.gaps.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{PolicyProfile, Q16};

    fn policy() -> PolicyProfile {
        PolicyProfile::default_report_only()
    }

    fn mid() -> Digest {
        Digest::of_bytes(b"manifest")
    }

    fn hid() -> Digest {
        Digest::of_bytes(b"host")
    }

    #[test]
    fn compatibility_report_is_content_addressed() {
        let r1 = CompatibilityProfileReport::perfect(mid(), hid(), &policy());
        let r2 = CompatibilityProfileReport::perfect(mid(), hid(), &policy());
        assert_eq!(r1.report_id, r2.report_id);
        assert_ne!(r1.report_id, Digest::ZERO);
    }

    #[test]
    fn compatibility_no_host_snapshot_has_zero_score() {
        let r = CompatibilityProfileReport::no_host_snapshot(mid(), &policy());
        assert_eq!(r.status, CompatibilityStatus::NoHostSnapshot);
        assert_eq!(r.compatibility_score, Q16::ZERO);
    }

    #[test]
    fn compatibility_perfect_has_one_score_and_no_gaps() {
        let r = CompatibilityProfileReport::perfect(mid(), hid(), &policy());
        assert_eq!(r.status, CompatibilityStatus::Available);
        assert_eq!(r.compatibility_score, Q16::ONE);
        assert!(r.gaps.is_empty());
    }

    #[test]
    fn compatibility_gaps_sorted_by_unit_id() {
        let u1 = Digest::of_bytes(b"u1");
        let u2 = Digest::of_bytes(b"u2");
        let gaps = vec![
            CompatibilityGap { unit_id: u2, gap_kind: "B".into(), severity: Q16::from_raw(10) },
            CompatibilityGap { unit_id: u1, gap_kind: "A".into(), severity: Q16::from_raw(20) },
        ];
        let r = CompatibilityProfileReport::new(
            mid(), hid(), &policy(),
            CompatibilityStatus::Available,
            Q16::HALF, gaps,
        );
        let ids: Vec<Digest> = r.gaps.iter().map(|g| g.unit_id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "gaps must be sorted by unit_id");
    }

    #[test]
    fn compatibility_summary_non_empty() {
        let r = CompatibilityProfileReport::perfect(mid(), hid(), &policy());
        let s = r.summary();
        assert!(s.contains("CompatibilityProfileReport"));
    }
}

//! Contradiction energy report for a SystemCube.
//!
//! Contradiction energy is a Q16-scaled metric that counts weighted structural
//! contradictions between `BlueprintUnit`s in a manifest. High energy indicates
//! that units conflict — they should not be materialised together.
//!
//! The report is advisory only. It cannot authorise materialization.

use kosmo_core::{Digest, PolicyProfile, Q16};
use serde::{Deserialize, Serialize};

// ─── Contradiction Record ─────────────────────────────────────────────────────

/// A pairwise contradiction between two blueprint units.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContradictionRecord {
    pub unit_a_id: Digest,
    pub unit_b_id: Digest,
    /// Q16-scaled weight of this contradiction.
    pub weight: Q16,
    pub reason: String,
}

// ─── Computation Status ───────────────────────────────────────────────────────

/// Whether contradiction energy could be computed for this manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnergyStatus {
    /// Energy was computed successfully.
    Available,
    /// Insufficient units to compute meaningful energy (< 2 accepted units).
    Insufficient,
    /// Energy computation was skipped by policy (e.g. ReportOnly with no units).
    Skipped,
}

// ─── Contradiction Energy Report ──────────────────────────────────────────────

/// Content for deterministic `ContradictionEnergyReport` ID.
#[derive(Serialize)]
struct EnergyReportContent {
    manifest_id: Digest,
    policy_id: Digest,
    status: String,
    total_energy_raw: i64,
    contradiction_count: u32,
}

/// Advisory contradiction energy report for a `SystemCubeManifest`.
///
/// Energy is the Q16-sum of contradiction weights.
/// A high energy value is a diagnostic signal — not an authoritative block.
/// Materialization must still pass Foundry and operator approval.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContradictionEnergyReport {
    pub report_id: Digest,
    pub manifest_id: Digest,
    pub policy_id: Digest,
    pub status: EnergyStatus,
    /// Q16 sum of all contradiction weights.
    pub total_energy: Q16,
    /// Sorted by (unit_a_id, unit_b_id) for determinism.
    pub contradictions: Vec<ContradictionRecord>,
}

impl ContradictionEnergyReport {
    pub fn new(
        manifest_id: Digest,
        policy: &PolicyProfile,
        status: EnergyStatus,
        mut contradictions: Vec<ContradictionRecord>,
    ) -> Self {
        contradictions.sort_by_key(|c| (c.unit_a_id, c.unit_b_id));
        let total_energy = contradictions
            .iter()
            .fold(Q16::ZERO, |acc, c| acc.saturating_add(c.weight));
        let report_id = Digest::of(&EnergyReportContent {
            manifest_id,
            policy_id: policy.id,
            status: format!("{:?}", status),
            total_energy_raw: total_energy.raw(),
            contradiction_count: contradictions.len() as u32,
        });
        Self {
            report_id,
            manifest_id,
            policy_id: policy.id,
            status,
            total_energy,
            contradictions,
        }
    }

    /// Build an `Insufficient` report when the manifest has fewer than 2 accepted units.
    pub fn insufficient(manifest_id: Digest, policy: &PolicyProfile) -> Self {
        Self::new(manifest_id, policy, EnergyStatus::Insufficient, vec![])
    }

    /// Build a zero-energy `Available` report for a contradiction-free manifest.
    pub fn zero_energy(manifest_id: Digest, policy: &PolicyProfile) -> Self {
        Self::new(manifest_id, policy, EnergyStatus::Available, vec![])
    }

    pub fn summary(&self) -> String {
        format!(
            "ContradictionEnergyReport — status={:?} energy={} contradictions={}",
            self.status,
            self.total_energy.raw(),
            self.contradictions.len(),
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

    #[test]
    fn energy_report_is_content_addressed() {
        let r1 = ContradictionEnergyReport::zero_energy(mid(), &policy());
        let r2 = ContradictionEnergyReport::zero_energy(mid(), &policy());
        assert_eq!(r1.report_id, r2.report_id);
        assert_ne!(r1.report_id, Digest::ZERO);
    }

    #[test]
    fn energy_report_insufficient_has_zero_energy() {
        let r = ContradictionEnergyReport::insufficient(mid(), &policy());
        assert_eq!(r.status, EnergyStatus::Insufficient);
        assert_eq!(r.total_energy, Q16::ZERO);
        assert!(r.contradictions.is_empty());
    }

    #[test]
    fn energy_report_sums_contradiction_weights() {
        let a = Digest::of_bytes(b"a");
        let b = Digest::of_bytes(b"b");
        let contradictions = vec![
            ContradictionRecord {
                unit_a_id: a,
                unit_b_id: b,
                weight: Q16::from_raw(300),
                reason: "test".into(),
            },
            ContradictionRecord {
                unit_a_id: b,
                unit_b_id: a,
                weight: Q16::from_raw(200),
                reason: "test2".into(),
            },
        ];
        let r = ContradictionEnergyReport::new(mid(), &policy(), EnergyStatus::Available, contradictions);
        assert_eq!(r.total_energy.raw(), 500);
    }

    #[test]
    fn energy_report_contradictions_sorted() {
        let a = Digest::of_bytes(b"a");
        let b = Digest::of_bytes(b"b");
        let c = Digest::of_bytes(b"c");
        let contradictions = vec![
            ContradictionRecord { unit_a_id: b, unit_b_id: c, weight: Q16::from_raw(1), reason: "".into() },
            ContradictionRecord { unit_a_id: a, unit_b_id: b, weight: Q16::from_raw(1), reason: "".into() },
        ];
        let r = ContradictionEnergyReport::new(mid(), &policy(), EnergyStatus::Available, contradictions);
        let keys: Vec<(Digest, Digest)> = r.contradictions.iter().map(|c| (c.unit_a_id, c.unit_b_id)).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "contradictions must be sorted by (unit_a_id, unit_b_id)");
    }

    #[test]
    fn energy_report_summary_non_empty() {
        let r = ContradictionEnergyReport::zero_energy(mid(), &policy());
        let s = r.summary();
        assert!(s.contains("ContradictionEnergyReport"));
        assert!(s.contains("Available"));
    }
}

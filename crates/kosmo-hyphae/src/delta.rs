use crate::cube::SourceCube;
use kosmo_core::{rank_by_energy, Digest, FoundrySurvival, GateResult, LicenseStatus, Q16};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The planned action for one void in the HostTargetDelta.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeltaAction {
    /// A source cube can fill this void (planning only — no host mutation).
    FillVoid { top_candidate_cube_id: Digest },
    /// Deficiency can be reduced but not fully eliminated.
    ReduceDeficiency { reduction_score: i64 },
    /// No available source cube addresses this void.
    Unresolvable,
}

/// The delta entry for one host void.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoidFillDelta {
    pub void_id: Digest,
    /// All source cube candidates that target this void (sorted).
    pub candidate_cube_ids: Vec<Digest>,
    pub best_support_score: Q16,
    pub action: DeltaAction,
}

/// Overall planability status of a HostTargetDelta.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeltaStatus {
    /// No voids to fill — host is already clean.
    Clean,
    /// All voids have at least one candidate.
    FullyPlannable,
    /// Some voids have candidates; others do not.
    PartiallyPlannable { plannable: u64, unresolvable: u64 },
    /// No voids have any candidates.
    Unresolvable,
}

/// Serialize-only for HostTargetDelta content-addressing.
#[derive(Serialize)]
struct DeltaContent {
    host_cube_id: Digest,
    composite_cube_id: Digest,
    void_fill_count: u64,
    remaining_void_count: u64,
    policy_id: Digest,
}

/// Planning-only delta between current host void state and available source cubes.
///
/// Describes which voids could be filled and by which source cubes.
/// No host mutations are authorized or performed (HYPHAE v0.3 passive —
/// materialization is Phase 11 and requires explicit operator authorization).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostTargetDelta {
    pub delta_id: Digest,
    pub host_cube_id: Digest,
    pub composite_cube_id: Digest,
    pub void_fills: Vec<VoidFillDelta>,
    pub remaining_voids: Vec<Digest>,
    pub status: DeltaStatus,
    pub policy_id: Digest,
}

impl HostTargetDelta {
    pub fn new(
        host_cube_id: Digest,
        composite_cube_id: Digest,
        void_fills: Vec<VoidFillDelta>,
        remaining_voids: Vec<Digest>,
        policy_id: Digest,
    ) -> Self {
        let status = Self::compute_status(&void_fills, &remaining_voids);
        let delta_id = Digest::of(&DeltaContent {
            host_cube_id,
            composite_cube_id,
            void_fill_count: void_fills.len() as u64,
            remaining_void_count: remaining_voids.len() as u64,
            policy_id,
        });
        Self {
            delta_id,
            host_cube_id,
            composite_cube_id,
            void_fills,
            remaining_voids,
            status,
            policy_id,
        }
    }

    /// Build a `HostTargetDelta` from real `SourceCube` objects using the
    /// tripolar energy kernel for ranking — the energy-correct path.
    ///
    /// Each cube's energy is computed via [`SourceCube::energy_assessment`]
    /// and candidates per void are ranked by [`rank_by_energy`]. The cube with
    /// the highest energy wins; ties are broken by `subject_id` for determinism.
    ///
    /// `seam_map`: per-void seam coherence scores from LPCM analysis
    /// (`void_id → Q16`). Missing entries default to `Q16::ONE` (no penalty).
    /// Gate defaults to `Pass`; license defaults to `NotApplicable`;
    /// foundry defaults to `Unavailable` (no Foundry run at planning stage).
    pub fn from_source_cubes(
        host_cube_id: Digest,
        host_void_ids: &[Digest],
        composite_cube_id: Digest,
        cubes: &[SourceCube],
        seam_map: &BTreeMap<Digest, Q16>,
        policy_id: Digest,
    ) -> Self {
        let mut void_fills = Vec::new();
        let mut remaining_voids = Vec::new();

        for &void_id in host_void_ids {
            let seam_coherence = seam_map.get(&void_id).copied().unwrap_or(Q16::ONE);
            let cube_assessments: Vec<_> = cubes
                .iter()
                .filter(|c| c.target_void_id == Some(void_id))
                .map(|c| {
                    let a = c.energy_assessment(
                        &GateResult::Pass,
                        &LicenseStatus::NotApplicable,
                        FoundrySurvival::Unavailable,
                        seam_coherence,
                    );
                    (c.cube_id, a)
                })
                .collect();

            if cube_assessments.is_empty() {
                remaining_voids.push(void_id);
            } else {
                let assessments: Vec<_> = cube_assessments.iter().map(|(_, a)| a.clone()).collect();
                let ranked = rank_by_energy(&assessments);
                let top = ranked.first().unwrap();
                let top_cube_id = cube_assessments
                    .iter()
                    .find(|(_, a)| a.id == top.id)
                    .map(|(cid, _)| *cid)
                    .unwrap();
                let mut candidate_cube_ids: Vec<Digest> =
                    cube_assessments.iter().map(|(cid, _)| *cid).collect();
                candidate_cube_ids.sort();
                void_fills.push(VoidFillDelta {
                    void_id,
                    candidate_cube_ids,
                    best_support_score: top.energy,
                    action: DeltaAction::FillVoid { top_candidate_cube_id: top_cube_id },
                });
            }
        }

        Self::new(host_cube_id, composite_cube_id, void_fills, remaining_voids, policy_id)
    }

    pub fn plannable_count(&self) -> usize {
        self.void_fills
            .iter()
            .filter(|vf| !matches!(vf.action, DeltaAction::Unresolvable))
            .count()
    }

    pub fn summary(&self) -> String {
        format!(
            "HostTargetDelta (report-only) — fills: {} | remaining: {} | status: {:?}",
            self.void_fills.len(),
            self.remaining_voids.len(),
            self.status,
        )
    }

    fn compute_status(
        void_fills: &[VoidFillDelta],
        remaining_voids: &[Digest],
    ) -> DeltaStatus {
        match (void_fills.is_empty(), remaining_voids.is_empty()) {
            (true, true) => DeltaStatus::Clean,
            (false, true) => DeltaStatus::FullyPlannable,
            (true, false) => DeltaStatus::Unresolvable,
            (false, false) => DeltaStatus::PartiallyPlannable {
                plannable: void_fills.len() as u64,
                unresolvable: remaining_voids.len() as u64,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube::{CubeDimensionProfile, SourceCube};
    use kosmo_core::{Digest, TaintLabel, Q16};

    #[test]
    fn delta_clean_for_no_voids() {
        let pid = Digest::of_bytes(b"p");
        let delta = HostTargetDelta::new(Digest::ZERO, Digest::ZERO, vec![], vec![], pid);
        assert_eq!(delta.status, DeltaStatus::Clean);
        assert_ne!(delta.delta_id, Digest::ZERO);
    }

    #[test]
    fn delta_fully_plannable_when_all_voids_covered() {
        let pid = Digest::of_bytes(b"p");
        let void_id = Digest::of_bytes(b"v");
        let cube_id = Digest::of_bytes(b"c");
        let fills = vec![VoidFillDelta {
            void_id,
            candidate_cube_ids: vec![cube_id],
            best_support_score: Q16::HALF,
            action: DeltaAction::FillVoid { top_candidate_cube_id: cube_id },
        }];
        let delta = HostTargetDelta::new(Digest::ZERO, Digest::ZERO, fills, vec![], pid);
        assert_eq!(delta.status, DeltaStatus::FullyPlannable);
        assert_eq!(delta.plannable_count(), 1);
    }

    #[test]
    fn delta_is_content_addressed() {
        let pid = Digest::of_bytes(b"p");
        let d1 = HostTargetDelta::new(Digest::ZERO, Digest::ZERO, vec![], vec![], pid);
        let d2 = HostTargetDelta::new(Digest::ZERO, Digest::ZERO, vec![], vec![], pid);
        assert_eq!(d1.delta_id, d2.delta_id);
        assert_ne!(d1.delta_id, Digest::ZERO);
    }

    fn make_cube(void_id: Digest, support: Q16, seed: &[u8], pid: Digest) -> SourceCube {
        SourceCube::new(
            Some(void_id),
            std::str::from_utf8(seed).unwrap_or("cube").to_string(),
            CubeDimensionProfile::empty(),
            support,
            TaintLabel::Clean,
            Digest::of_bytes(b"ev"),
            pid,
        )
    }

    #[test]
    fn delta_from_source_cubes_fills_matching_voids() {
        let pid = Digest::of_bytes(b"p");
        let void_id = Digest::of_bytes(b"v");
        let cube = make_cube(void_id, Q16::HALF, b"cube", pid);
        let cube_id = cube.cube_id;

        let delta = HostTargetDelta::from_source_cubes(
            Digest::of_bytes(b"h"),
            &[void_id],
            Digest::of_bytes(b"comp"),
            &[cube],
            &BTreeMap::new(),
            pid,
        );

        assert_eq!(delta.void_fills.len(), 1);
        assert!(delta.remaining_voids.is_empty());
        assert_eq!(delta.status, DeltaStatus::FullyPlannable);
        assert_eq!(
            delta.void_fills[0].action,
            DeltaAction::FillVoid { top_candidate_cube_id: cube_id }
        );
    }

    #[test]
    fn delta_partially_plannable_for_mixed_voids() {
        let pid = Digest::of_bytes(b"p");
        let void1 = Digest::of_bytes(b"v1");
        let void2 = Digest::of_bytes(b"v2"); // no cube covers this
        let cube = make_cube(void1, Q16::HALF, b"cube", pid);

        let delta = HostTargetDelta::from_source_cubes(
            Digest::ZERO,
            &[void1, void2],
            Digest::ZERO,
            &[cube],
            &BTreeMap::new(),
            pid,
        );

        assert_eq!(delta.void_fills.len(), 1);
        assert_eq!(delta.remaining_voids.len(), 1);
        assert!(matches!(
            delta.status,
            DeltaStatus::PartiallyPlannable { plannable: 1, unresolvable: 1 }
        ));
    }

    #[test]
    fn delta_summary_non_empty() {
        let pid = Digest::of_bytes(b"p");
        let delta = HostTargetDelta::new(Digest::ZERO, Digest::ZERO, vec![], vec![], pid);
        let s = delta.summary();
        assert!(s.contains("HostTargetDelta"));
        assert!(s.contains("report-only"));
    }
}

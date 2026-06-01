use crate::cube::SourceCube;
use kosmo_core::{rank_by_energy, Digest, FoundrySurvival, GateResult, LicenseStatus, Q16};
use serde::{Deserialize, Serialize};

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

    /// Build a HostTargetDelta by matching host voids against source cube targets.
    ///
    /// `source_cube_targets`: `(cube_id, target_void_id, support_score)` triples.
    /// Planning only — emitted as a report-only artifact.
    pub fn from_host_and_composite(
        host_cube_id: Digest,
        host_void_ids: &[Digest],
        composite_cube_id: Digest,
        source_cube_targets: &[(Digest, Digest, Q16)],
        policy_id: Digest,
    ) -> Self {
        let mut void_fills = Vec::new();
        let mut remaining_voids = Vec::new();

        for &void_id in host_void_ids {
            let mut candidates: Vec<(Digest, Q16)> = source_cube_targets
                .iter()
                .filter(|(_, tvid, _)| *tvid == void_id)
                .map(|(cid, _, score)| (*cid, *score))
                .collect();
            // Sort by cube_id for determinism.
            candidates.sort_by_key(|(id, _)| *id);

            if candidates.is_empty() {
                remaining_voids.push(void_id);
            } else {
                let best = *candidates.iter().max_by_key(|(_, s)| s.raw()).unwrap();
                let mut candidate_cube_ids: Vec<Digest> =
                    candidates.iter().map(|(id, _)| *id).collect();
                candidate_cube_ids.sort();
                void_fills.push(VoidFillDelta {
                    void_id,
                    candidate_cube_ids,
                    best_support_score: best.1,
                    action: DeltaAction::FillVoid { top_candidate_cube_id: best.0 },
                });
            }
        }

        Self::new(host_cube_id, composite_cube_id, void_fills, remaining_voids, policy_id)
    }

    /// Build a `HostTargetDelta` from real `SourceCube` objects using the
    /// tripolar energy kernel for ranking — the energy-correct path.
    ///
    /// Each cube's energy is computed via [`SourceCube::energy_assessment`]
    /// and candidates per void are ranked by [`rank_by_energy`]. The cube with
    /// the highest energy wins; ties are broken by `subject_id` for determinism.
    ///
    /// Gate defaults to `Pass` (cubes in the swarm already passed intake);
    /// license defaults to `NotApplicable` (planning context);
    /// foundry defaults to `Unavailable` (no Foundry run at planning stage).
    pub fn from_source_cubes(
        host_cube_id: Digest,
        host_void_ids: &[Digest],
        composite_cube_id: Digest,
        cubes: &[SourceCube],
        policy_id: Digest,
    ) -> Self {
        let mut void_fills = Vec::new();
        let mut remaining_voids = Vec::new();

        for &void_id in host_void_ids {
            let cube_assessments: Vec<_> = cubes
                .iter()
                .filter(|c| c.target_void_id == Some(void_id))
                .map(|c| {
                    let a = c.energy_assessment(
                        &GateResult::Pass,
                        &LicenseStatus::NotApplicable,
                        FoundrySurvival::Unavailable,
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
    use kosmo_core::{Digest, Q16};

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

    #[test]
    fn delta_from_host_and_composite_fills_matching_voids() {
        let pid = Digest::of_bytes(b"p");
        let void_id = Digest::of_bytes(b"v");
        let cube_id = Digest::of_bytes(b"cube");

        let delta = HostTargetDelta::from_host_and_composite(
            Digest::of_bytes(b"h"),
            &[void_id],
            Digest::of_bytes(b"comp"),
            &[(cube_id, void_id, Q16::HALF)],
            pid,
        );

        assert_eq!(delta.void_fills.len(), 1);
        assert!(delta.remaining_voids.is_empty());
        assert_eq!(delta.status, DeltaStatus::FullyPlannable);
        assert_eq!(delta.void_fills[0].best_support_score, Q16::HALF);
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
        let cube_id = Digest::of_bytes(b"cube");

        let delta = HostTargetDelta::from_host_and_composite(
            Digest::ZERO,
            &[void1, void2],
            Digest::ZERO,
            &[(cube_id, void1, Q16::HALF)],
            pid,
        );

        assert_eq!(delta.void_fills.len(), 1);
        assert_eq!(delta.remaining_voids.len(), 1);
        assert!(matches!(delta.status, DeltaStatus::PartiallyPlannable { plannable: 1, unresolvable: 1 }));
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

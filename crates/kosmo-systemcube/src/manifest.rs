//! SystemCubeManifest — canonical package descriptor for a SystemCube export.
//!
//! The manifest is the stable, round-trippable record of a SystemCube export.
//! Its `manifest_id` is a deterministic content-addressed digest of all
//! accepted `BlueprintUnit` IDs, the host snapshot reference, and the policy.

use crate::blueprint_unit::BlueprintUnit;
use kosmo_core::{Digest, PolicyProfile, RunDescriptor};
use serde::{Deserialize, Serialize};

// ─── Manifest ─────────────────────────────────────────────────────────────────

/// Content for deterministic `SystemCubeManifest` ID.
#[derive(Serialize)]
struct ManifestContent {
    host_snapshot_id: Digest,
    run_id: Digest,
    policy_id: Digest,
    accepted_unit_ids: Vec<Digest>,
    rejected_unit_count: u32,
}

/// Canonical manifest of a `SystemCube` export.
///
/// Contains only the accepted, evidence-bound `BlueprintUnit` IDs.
/// Rejected (opaque) units are counted but excluded from the accepted list.
/// The `manifest_id` is stable and round-trips through serialisation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemCubeManifest {
    pub manifest_id: Digest,
    pub host_snapshot_id: Digest,
    pub run_id: Digest,
    pub policy_id: Digest,
    /// IDs of accepted blueprint units, sorted for determinism.
    pub accepted_unit_ids: Vec<Digest>,
    /// Count of rejected (opaque) units excluded from this manifest.
    pub rejected_unit_count: u32,
}

impl SystemCubeManifest {
    pub fn new(
        host_snapshot_id: Digest,
        run: &RunDescriptor,
        policy: &PolicyProfile,
        units: &[BlueprintUnit],
    ) -> Self {
        let mut accepted_unit_ids: Vec<Digest> = units
            .iter()
            .filter(|u| u.is_accepted())
            .map(|u| u.unit_id)
            .collect();
        accepted_unit_ids.sort();

        let rejected_unit_count =
            units.iter().filter(|u| !u.is_accepted()).count() as u32;

        let manifest_id = Digest::of(&ManifestContent {
            host_snapshot_id,
            run_id: run.run_id,
            policy_id: policy.id,
            accepted_unit_ids: accepted_unit_ids.clone(),
            rejected_unit_count,
        });

        Self {
            manifest_id,
            host_snapshot_id,
            run_id: run.run_id,
            policy_id: policy.id,
            accepted_unit_ids,
            rejected_unit_count,
        }
    }

    /// Number of accepted units in this manifest.
    pub fn accepted_count(&self) -> usize {
        self.accepted_unit_ids.len()
    }

    /// Whether the manifest is empty (no accepted units).
    pub fn is_empty(&self) -> bool {
        self.accepted_unit_ids.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprint_unit::{BlueprintUnit, BlueprintUnitKind};
    use kosmo_core::{AuthorityLabel, PolicyProfile, RunDescriptor, TaintLabel};

    fn policy() -> PolicyProfile {
        PolicyProfile::default_report_only()
    }

    fn run() -> RunDescriptor {
        RunDescriptor::new(policy().id, "test-host")
    }

    fn host_snap() -> Digest {
        Digest::of_bytes(b"host_snapshot")
    }

    fn make_accepted_unit() -> BlueprintUnit {
        BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            Digest::of_bytes(b"src"),
            AuthorityLabel::Operator,
            TaintLabel::Clean,
            vec![Digest::of_bytes(b"ev")],
            &policy(),
        )
    }

    fn make_rejected_unit() -> BlueprintUnit {
        BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            Digest::of_bytes(b"src2"),
            AuthorityLabel::Operator,
            TaintLabel::Clean,
            vec![], // no evidence → rejected opaque
            &policy(),
        )
    }

    #[test]
    fn manifest_is_content_addressed() {
        let units = vec![make_accepted_unit()];
        let m1 = SystemCubeManifest::new(host_snap(), &run(), &policy(), &units);
        let m2 = SystemCubeManifest::new(host_snap(), &run(), &policy(), &units);
        assert_eq!(m1.manifest_id, m2.manifest_id);
        assert_ne!(m1.manifest_id, Digest::ZERO);
    }

    #[test]
    fn manifest_excludes_rejected_units() {
        let units = vec![make_accepted_unit(), make_rejected_unit()];
        let m = SystemCubeManifest::new(host_snap(), &run(), &policy(), &units);
        assert_eq!(m.accepted_count(), 1);
        assert_eq!(m.rejected_unit_count, 1);
    }

    #[test]
    fn manifest_accepted_unit_ids_sorted() {
        let u1 = make_accepted_unit();
        let u2 = BlueprintUnit::new(
            BlueprintUnitKind::FiberDescriptor,
            Digest::of_bytes(b"src3"),
            AuthorityLabel::Operator,
            TaintLabel::Clean,
            vec![Digest::of_bytes(b"ev3")],
            &policy(),
        );
        let m = SystemCubeManifest::new(host_snap(), &run(), &policy(), &[u1, u2]);
        let ids = &m.accepted_unit_ids;
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(*ids, sorted, "accepted_unit_ids must be sorted");
    }

    #[test]
    fn manifest_round_trips_via_json() {
        let units = vec![make_accepted_unit()];
        let m = SystemCubeManifest::new(host_snap(), &run(), &policy(), &units);
        let json = serde_json::to_string(&m).expect("serialize");
        let m2: SystemCubeManifest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(m.manifest_id, m2.manifest_id, "manifest_id must survive round-trip");
    }

    #[test]
    fn manifest_empty_when_all_units_rejected() {
        let units = vec![make_rejected_unit()];
        let m = SystemCubeManifest::new(host_snap(), &run(), &policy(), &units);
        assert!(m.is_empty());
        assert_eq!(m.rejected_unit_count, 1);
    }
}

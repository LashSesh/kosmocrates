use kosmo_core::{
    rank_by_energy, Digest, EnergyAssessment, EnergyFactors, EnergyKernel, FoundrySurvival,
    GateResult, LicenseStatus, TripolarEnergy, Q16,
};
use serde::{Deserialize, Serialize};

/// A named structural gap in the host topology.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostVoidKind {
    MissingTestFiber { for_module: String },
    MissingDocFiber { for_module: String },
    MissingImplementation { intent: String },
    MissingErrorHandling { location: String },
    MissingTypeAnnotation { location: String },
    IncompleteFunctionBody { location: String },
    Custom { description: String },
}

/// Serialize-only for content-addressing.
#[derive(Serialize)]
struct VoidContent<'a> {
    kind: &'a HostVoidKind,
    severity: Q16,
    location: &'a str,
}

/// A single topological void in the host codebase.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostVoid {
    pub void_id: Digest,
    pub kind: HostVoidKind,
    /// Severity in `[0, 1]` as Q16 — no floats in gate paths.
    pub severity: Q16,
    pub location: String,
}

impl HostVoid {
    pub fn new(kind: HostVoidKind, severity: Q16, location: String) -> Self {
        let void_id = Digest::of(&VoidContent {
            kind: &kind,
            severity,
            location: &location,
        });
        Self {
            void_id,
            kind,
            severity,
            location,
        }
    }

    pub fn verify_id(&self) -> bool {
        let expected = Digest::of(&VoidContent {
            kind: &self.kind,
            severity: self.severity,
            location: &self.location,
        });
        self.void_id == expected
    }

    /// Build an [`EnergyAssessment`] for this void, ranking it by urgency.
    ///
    /// - ψ (meaning)  = `severity` — how urgent this void is (Q16 in `[0,1]`).
    /// - ρ (coherence) = `Q16::ONE` — no structural coherence data at void level.
    /// - ω (phase)    = `Q16::ONE` — no phase data at void level.
    /// - `evidence_bundle_id` = `void_id`: the void's own content address,
    ///   derived from `{kind, severity, location}` during workspace scan.
    ///   Satisfies CROSS-006 (non-ZERO evidence ref).
    pub fn energy_assessment(&self, gate: &GateResult, policy_id: Digest) -> EnergyAssessment {
        let tripolar = TripolarEnergy::new(self.severity, Q16::ONE, Q16::ONE);
        let factors = EnergyFactors {
            gate: EnergyFactors::gate_factor(gate),
            taint: Q16::ONE,
            license: EnergyFactors::license_factor(&LicenseStatus::NotApplicable),
            foundry: EnergyFactors::foundry_factor(FoundrySurvival::Unavailable),
            seam: Q16::ONE,
            contradiction: Q16::ONE,
        };
        EnergyAssessment::new(
            self.void_id,
            EnergyKernel::new(tripolar, factors),
            policy_id,
            self.void_id, // evidence = void_id (content-addresses the detection)
        )
    }
}

/// Serialize-only for content-addressing TopologicalVoidMap.
#[derive(Serialize)]
struct VoidMapContent {
    voids: Vec<Digest>,
    policy_id: Digest,
}

/// The complete map of structural voids in a host codebase.
///
/// `voids` are sorted by `void_id` for determinism.
/// `map_id` is the content-addressed digest of the sorted void IDs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopologicalVoidMap {
    pub map_id: Digest,
    pub voids: Vec<HostVoid>,
    pub policy_id: Digest,
}

impl TopologicalVoidMap {
    pub fn empty(policy_id: Digest) -> Self {
        let map_id = Digest::of(&VoidMapContent {
            voids: vec![],
            policy_id,
        });
        Self {
            map_id,
            voids: vec![],
            policy_id,
        }
    }

    pub fn from_voids(mut voids: Vec<HostVoid>, policy_id: Digest) -> Self {
        voids.sort_by_key(|v| v.void_id);
        let map_id = Digest::of(&VoidMapContent {
            voids: voids.iter().map(|v| v.void_id).collect(),
            policy_id,
        });
        Self {
            map_id,
            voids,
            policy_id,
        }
    }

    pub fn void_count(&self) -> usize {
        self.voids.len()
    }

    pub fn count_by_kind<F: Fn(&HostVoidKind) -> bool>(&self, pred: F) -> usize {
        self.voids.iter().filter(|v| pred(&v.kind)).count()
    }

    pub fn verify_id(&self) -> bool {
        let expected = Digest::of(&VoidMapContent {
            voids: self.voids.iter().map(|v| v.void_id).collect(),
            policy_id: self.policy_id,
        });
        self.map_id == expected
    }

    /// Return void IDs ranked by severity (highest first) using the energy kernel.
    ///
    /// Ties are broken deterministically by `void_id` (content address). An
    /// empty map returns an empty vec.
    pub fn priority_ranking(&self, gate: &GateResult) -> Vec<Digest> {
        let assessments: Vec<EnergyAssessment> = self
            .voids
            .iter()
            .map(|v| v.energy_assessment(gate, self.policy_id))
            .collect();
        rank_by_energy(&assessments)
            .into_iter()
            .map(|a| a.subject_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::Q16;

    #[test]
    fn host_void_id_deterministic() {
        let v1 = HostVoid::new(
            HostVoidKind::MissingTestFiber {
                for_module: "src/foo.rs".into(),
            },
            Q16::HALF,
            "src/foo.rs".into(),
        );
        let v2 = HostVoid::new(
            HostVoidKind::MissingTestFiber {
                for_module: "src/foo.rs".into(),
            },
            Q16::HALF,
            "src/foo.rs".into(),
        );
        assert_eq!(v1.void_id, v2.void_id);
        assert!(v1.verify_id());
    }

    #[test]
    fn void_map_sorted_deterministic() {
        let pid = Digest::of_bytes(b"p");
        let v1 = HostVoid::new(
            HostVoidKind::Custom {
                description: "z".into(),
            },
            Q16::HALF,
            "z".into(),
        );
        let v2 = HostVoid::new(
            HostVoidKind::Custom {
                description: "a".into(),
            },
            Q16::HALF,
            "a".into(),
        );
        let m1 = TopologicalVoidMap::from_voids(vec![v1.clone(), v2.clone()], pid);
        let m2 = TopologicalVoidMap::from_voids(vec![v2, v1], pid);
        assert_eq!(
            m1.map_id, m2.map_id,
            "map_id must be stable regardless of insertion order"
        );
        assert!(m1.verify_id());
    }

    #[test]
    fn void_map_empty() {
        let vm = TopologicalVoidMap::empty(Digest::ZERO);
        assert_eq!(vm.void_count(), 0);
        assert!(vm.verify_id());
    }

    #[test]
    fn host_void_energy_assessment_content_addressed() {
        let pid = Digest::of_bytes(b"p");
        let v = HostVoid::new(
            HostVoidKind::Custom {
                description: "gap".into(),
            },
            Q16::HALF,
            "x".into(),
        );
        let a1 = v.energy_assessment(&GateResult::Pass, pid);
        let a2 = v.energy_assessment(&GateResult::Pass, pid);
        assert_eq!(a1.id, a2.id, "energy_assessment must be deterministic");
        assert_eq!(a1.subject_id, v.void_id);
        assert_eq!(
            a1.evidence_bundle_id, v.void_id,
            "evidence_bundle_id must equal void_id"
        );
        assert_ne!(
            a1.evidence_bundle_id,
            Digest::ZERO,
            "CROSS-006: non-ZERO evidence ref"
        );
    }

    #[test]
    fn host_void_reject_gate_zeroes_energy() {
        let pid = Digest::of_bytes(b"p");
        let v = HostVoid::new(
            HostVoidKind::Custom {
                description: "gap".into(),
            },
            Q16::ONE,
            "x".into(),
        );
        let a = v.energy_assessment(
            &GateResult::Reject {
                reason: "test".into(),
            },
            pid,
        );
        assert!(a.kernel.is_zeroed(), "Reject gate must zero void energy");
    }

    #[test]
    fn void_map_priority_ranking_orders_by_severity() {
        let pid = Digest::of_bytes(b"p");
        let high = HostVoid::new(
            HostVoidKind::Custom {
                description: "high".into(),
            },
            Q16::ONE,
            "h".into(),
        );
        let low = HostVoid::new(
            HostVoidKind::Custom {
                description: "low".into(),
            },
            Q16::ratio(1, 4).unwrap(),
            "l".into(),
        );
        let mid = HostVoid::new(
            HostVoidKind::Custom {
                description: "mid".into(),
            },
            Q16::HALF,
            "m".into(),
        );
        let high_id = high.void_id;
        let mid_id = mid.void_id;
        let low_id = low.void_id;
        let vm = TopologicalVoidMap::from_voids(vec![low, mid, high], pid);
        let ranking = vm.priority_ranking(&GateResult::Pass);
        assert_eq!(ranking.len(), 3);
        assert_eq!(ranking[0], high_id, "highest severity void must rank first");
        assert_eq!(ranking[1], mid_id, "mid severity void must rank second");
        assert_eq!(ranking[2], low_id, "lowest severity void must rank last");
    }

    #[test]
    fn void_map_priority_ranking_deterministic() {
        let pid = Digest::of_bytes(b"p");
        let v1 = HostVoid::new(
            HostVoidKind::Custom {
                description: "a".into(),
            },
            Q16::HALF,
            "a".into(),
        );
        let v2 = HostVoid::new(
            HostVoidKind::Custom {
                description: "b".into(),
            },
            Q16::HALF,
            "b".into(),
        );
        let vm = TopologicalVoidMap::from_voids(vec![v1, v2], pid);
        let r1 = vm.priority_ranking(&GateResult::Pass);
        let r2 = vm.priority_ranking(&GateResult::Pass);
        assert_eq!(r1, r2, "priority_ranking must be deterministic");
    }
}

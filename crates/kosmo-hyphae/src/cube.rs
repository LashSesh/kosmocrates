use crate::deficiency::DeficiencyKind;
use kosmo_core::{
    Digest, EnergyAssessment, EnergyFactors, EnergyKernel, FoundrySurvival, GateResult,
    LicenseStatus, TaintLabel, TripolarEnergy, Q16,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Serialize-only for CubeDimensionProfile content-addressing.
#[derive(Serialize)]
struct ProfileContent {
    /// Sorted (dimension_name, raw_score) pairs.
    dimensions: Vec<(String, i64)>,
}

/// The structural dimensions a cube covers, with per-dimension coverage scores.
///
/// Keys are `format!("{:?}", DeficiencyKind)` strings. BTreeMap serializes
/// in key order — deterministic and JCS-compatible.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CubeDimensionProfile {
    pub profile_id: Digest,
    pub dimensions: BTreeMap<String, Q16>,
}

impl CubeDimensionProfile {
    pub fn empty() -> Self {
        let profile_id = Self::compute_id(&BTreeMap::new());
        Self {
            profile_id,
            dimensions: BTreeMap::new(),
        }
    }

    pub fn from_raw_map(dimensions: BTreeMap<String, Q16>) -> Self {
        let profile_id = Self::compute_id(&dimensions);
        Self {
            profile_id,
            dimensions,
        }
    }

    pub fn with_dimension(mut self, kind: &DeficiencyKind, score: Q16) -> Self {
        self.dimensions.insert(format!("{:?}", kind), score);
        self.profile_id = Self::compute_id(&self.dimensions);
        self
    }

    pub fn coverage_for(&self, kind: &DeficiencyKind) -> Q16 {
        self.dimensions
            .get(&format!("{:?}", kind))
            .copied()
            .unwrap_or(Q16::ZERO)
    }

    fn compute_id(dimensions: &BTreeMap<String, Q16>) -> Digest {
        // BTreeMap iteration is in key order — deterministic.
        let dims: Vec<(String, i64)> = dimensions
            .iter()
            .map(|(k, v)| (k.clone(), v.raw()))
            .collect();
        Digest::of(&ProfileContent { dimensions: dims })
    }
}

/// Serialize-only for RepositoryCube content-addressing.
#[derive(Serialize)]
struct RepoCubeContent {
    source_path: String,
    dimension_profile_id: Digest,
    evidence_bundle_id: Digest,
    taint: String,
    policy_id: Digest,
}

/// A content-addressed record of a repository as a structural analysis unit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepositoryCube {
    pub cube_id: Digest,
    pub source_path: String,
    pub dimension_profile: CubeDimensionProfile,
    pub evidence_bundle_id: Digest,
    pub taint: TaintLabel,
    pub policy_id: Digest,
}

impl RepositoryCube {
    pub fn new(
        source_path: String,
        dimension_profile: CubeDimensionProfile,
        evidence_bundle_id: Digest,
        taint: TaintLabel,
        policy_id: Digest,
    ) -> Self {
        let cube_id = Digest::of(&RepoCubeContent {
            source_path: source_path.clone(),
            dimension_profile_id: dimension_profile.profile_id,
            evidence_bundle_id,
            taint: format!("{:?}", taint),
            policy_id,
        });
        Self {
            cube_id,
            source_path,
            dimension_profile,
            evidence_bundle_id,
            taint,
            policy_id,
        }
    }
}

/// Serialize-only for SourceCube content-addressing.
#[derive(Serialize)]
struct SourceCubeContent {
    target_void_id: Option<Digest>,
    source_path: String,
    dimension_profile_id: Digest,
    support_score: i64,
    taint: String,
    evidence_bundle_id: Digest,
    policy_id: Digest,
}

/// A source-side structural candidate for a void or deficiency.
///
/// Each SourceCube targets one void (or none, for general motif proposals).
/// Support score is Q16 — no floats in gate paths (CROSS-007).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceCube {
    pub cube_id: Digest,
    pub target_void_id: Option<Digest>,
    pub source_path: String,
    pub dimension_profile: CubeDimensionProfile,
    pub support_score: Q16,
    pub taint: TaintLabel,
    pub evidence_bundle_id: Digest,
    pub policy_id: Digest,
}

/// Average Q16 coverage across all dimensions in a profile.
/// Returns `Q16::ONE` when the profile is empty — unconstrained means no penalty.
fn average_profile_coverage(profile: &CubeDimensionProfile) -> Q16 {
    if profile.dimensions.is_empty() {
        return Q16::ONE;
    }
    let sum: i64 = profile.dimensions.values().map(|v| v.raw()).sum();
    Q16::from_raw(sum / profile.dimensions.len() as i64)
}

impl SourceCube {
    pub fn new(
        target_void_id: Option<Digest>,
        source_path: String,
        dimension_profile: CubeDimensionProfile,
        support_score: Q16,
        taint: TaintLabel,
        evidence_bundle_id: Digest,
        policy_id: Digest,
    ) -> Self {
        let cube_id = Digest::of(&SourceCubeContent {
            target_void_id,
            source_path: source_path.clone(),
            dimension_profile_id: dimension_profile.profile_id,
            support_score: support_score.raw(),
            taint: format!("{:?}", taint),
            evidence_bundle_id,
            policy_id,
        });
        Self {
            cube_id,
            target_void_id,
            source_path,
            dimension_profile,
            support_score,
            taint,
            evidence_bundle_id,
            policy_id,
        }
    }

    /// Build an [`EnergyAssessment`] for this cube using the tripolar kernel.
    ///
    /// - ψ (meaning) = `support_score` — goal-fit relevance to the targeted void.
    /// - ρ (coherence) = average dimension coverage from `dimension_profile`.
    /// - ω (phase) = `Q16::ONE` — no phase data at SourceCube level.
    /// - `seam_coherence` — LPCM seam compatibility score for the targeted void
    ///   (`Q16::ONE` when no seam analysis is available; lower = worse seam fit).
    /// - Taint factor derived from `self.taint`; gate/license/foundry from caller.
    ///
    /// Use `rank_by_energy` on the returned assessments to rank cubes by D.
    pub fn energy_assessment(
        &self,
        gate: &GateResult,
        license: &LicenseStatus,
        foundry: FoundrySurvival,
        seam_coherence: Q16,
    ) -> EnergyAssessment {
        let tripolar = TripolarEnergy::new(
            self.support_score,
            average_profile_coverage(&self.dimension_profile),
            Q16::ONE,
        );
        let factors = EnergyFactors {
            gate: EnergyFactors::gate_factor(gate),
            taint: EnergyFactors::taint_factor(&self.taint),
            license: EnergyFactors::license_factor(license),
            foundry: EnergyFactors::foundry_factor(foundry),
            seam: seam_coherence,
            contradiction: Q16::ONE,
        };
        EnergyAssessment::new(
            self.cube_id,
            EnergyKernel::new(tripolar, factors),
            self.policy_id,
            self.evidence_bundle_id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deficiency::DeficiencyKind;
    use kosmo_core::{Digest, TaintLabel, Q16};

    #[test]
    fn dimension_profile_from_raw_map_is_content_addressed() {
        let mut m = BTreeMap::new();
        m.insert("rho_coherence".to_string(), Q16::HALF);
        m.insert("omega_phase".to_string(), Q16::ratio(1, 4).unwrap());
        let p1 = CubeDimensionProfile::from_raw_map(m.clone());
        let p2 = CubeDimensionProfile::from_raw_map(m);
        assert_eq!(p1.profile_id, p2.profile_id);
        assert_ne!(p1.profile_id, Digest::ZERO);
        assert_eq!(p1.dimensions.len(), 2);
    }

    #[test]
    fn dimension_profile_empty_is_content_addressed() {
        let p = CubeDimensionProfile::empty();
        assert_ne!(p.profile_id, Digest::ZERO);
        let p2 = CubeDimensionProfile::empty();
        assert_eq!(p.profile_id, p2.profile_id);
    }

    #[test]
    fn dimension_profile_insertion_order_independent() {
        let p1 = CubeDimensionProfile::empty()
            .with_dimension(&DeficiencyKind::TestCoverage, Q16::HALF)
            .with_dimension(&DeficiencyKind::Documentation, Q16::ratio(1, 4).unwrap());
        let p2 = CubeDimensionProfile::empty()
            .with_dimension(&DeficiencyKind::Documentation, Q16::ratio(1, 4).unwrap())
            .with_dimension(&DeficiencyKind::TestCoverage, Q16::HALF);
        assert_eq!(
            p1.profile_id, p2.profile_id,
            "profile_id must be order-independent"
        );
    }

    #[test]
    fn dimension_profile_coverage_lookup() {
        let p =
            CubeDimensionProfile::empty().with_dimension(&DeficiencyKind::TestCoverage, Q16::HALF);
        assert_eq!(p.coverage_for(&DeficiencyKind::TestCoverage), Q16::HALF);
        assert_eq!(p.coverage_for(&DeficiencyKind::Documentation), Q16::ZERO);
    }

    #[test]
    fn source_cube_is_content_addressed() {
        let pid = Digest::of_bytes(b"p");
        let profile =
            CubeDimensionProfile::empty().with_dimension(&DeficiencyKind::TestCoverage, Q16::HALF);
        let s1 = SourceCube::new(
            Some(Digest::of_bytes(b"void")),
            "src/foo.rs".into(),
            profile.clone(),
            Q16::HALF,
            TaintLabel::Synthetic,
            Digest::ZERO,
            pid,
        );
        let s2 = SourceCube::new(
            Some(Digest::of_bytes(b"void")),
            "src/foo.rs".into(),
            profile,
            Q16::HALF,
            TaintLabel::Synthetic,
            Digest::ZERO,
            pid,
        );
        assert_eq!(s1.cube_id, s2.cube_id);
        assert_ne!(s1.cube_id, Digest::ZERO);
    }

    #[test]
    fn repository_cube_is_content_addressed() {
        let pid = Digest::of_bytes(b"p");
        let profile = CubeDimensionProfile::empty();
        let r1 = RepositoryCube::new(
            "repo/".into(),
            profile.clone(),
            Digest::ZERO,
            TaintLabel::Clean,
            pid,
        );
        let r2 = RepositoryCube::new(
            "repo/".into(),
            profile,
            Digest::ZERO,
            TaintLabel::Clean,
            pid,
        );
        assert_eq!(r1.cube_id, r2.cube_id);
        assert_ne!(r1.cube_id, Digest::ZERO);
    }
}

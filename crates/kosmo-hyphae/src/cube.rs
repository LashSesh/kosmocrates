use crate::deficiency::DeficiencyKind;
use kosmo_core::{Digest, Q16, TaintLabel};
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
        Self { profile_id, dimensions: BTreeMap::new() }
    }

    pub fn with_dimension(mut self, kind: &DeficiencyKind, score: Q16) -> Self {
        self.dimensions.insert(format!("{:?}", kind), score);
        self.profile_id = Self::compute_id(&self.dimensions);
        self
    }

    pub fn coverage_for(&self, kind: &DeficiencyKind) -> Q16 {
        self.dimensions.get(&format!("{:?}", kind)).copied().unwrap_or(Q16::ZERO)
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
        Self { cube_id, source_path, dimension_profile, evidence_bundle_id, taint, policy_id }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deficiency::DeficiencyKind;
    use kosmo_core::{Digest, Q16, TaintLabel};

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
        assert_eq!(p1.profile_id, p2.profile_id, "profile_id must be order-independent");
    }

    #[test]
    fn dimension_profile_coverage_lookup() {
        let p = CubeDimensionProfile::empty()
            .with_dimension(&DeficiencyKind::TestCoverage, Q16::HALF);
        assert_eq!(p.coverage_for(&DeficiencyKind::TestCoverage), Q16::HALF);
        assert_eq!(p.coverage_for(&DeficiencyKind::Documentation), Q16::ZERO);
    }

    #[test]
    fn source_cube_is_content_addressed() {
        let pid = Digest::of_bytes(b"p");
        let profile = CubeDimensionProfile::empty()
            .with_dimension(&DeficiencyKind::TestCoverage, Q16::HALF);
        let s1 = SourceCube::new(
            Some(Digest::of_bytes(b"void")), "src/foo.rs".into(),
            profile.clone(), Q16::HALF, TaintLabel::Synthetic, Digest::ZERO, pid,
        );
        let s2 = SourceCube::new(
            Some(Digest::of_bytes(b"void")), "src/foo.rs".into(),
            profile, Q16::HALF, TaintLabel::Synthetic, Digest::ZERO, pid,
        );
        assert_eq!(s1.cube_id, s2.cube_id);
        assert_ne!(s1.cube_id, Digest::ZERO);
    }

    #[test]
    fn repository_cube_is_content_addressed() {
        let pid = Digest::of_bytes(b"p");
        let profile = CubeDimensionProfile::empty();
        let r1 = RepositoryCube::new(
            "repo/".into(), profile.clone(), Digest::ZERO, TaintLabel::Clean, pid,
        );
        let r2 = RepositoryCube::new(
            "repo/".into(), profile, Digest::ZERO, TaintLabel::Clean, pid,
        );
        assert_eq!(r1.cube_id, r2.cube_id);
        assert_ne!(r1.cube_id, Digest::ZERO);
    }
}

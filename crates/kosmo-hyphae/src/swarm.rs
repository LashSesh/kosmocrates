use crate::cube::SourceCube;
use kosmo_core::{Digest, PolicyProfile, Q16};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Serialize-only for SourceCubeWorker content-addressing.
#[derive(Serialize)]
struct WorkerContent {
    cube_id: Digest,
    policy_id: Digest,
}

/// Processing status of a source cube worker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerStatus {
    Registered,
    Processed { yield_count: u64 },
    Blocked { reason: String },
}

/// A content-addressed record of one cube's processing slot in the swarm.
///
/// Workers are registered in cube_id order for deterministic replay.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceCubeWorker {
    pub worker_id: Digest,
    pub cube_id: Digest,
    pub status: WorkerStatus,
    pub policy_id: Digest,
}

impl SourceCubeWorker {
    pub fn for_cube(cube: &SourceCube, policy_id: Digest) -> Self {
        let worker_id = Digest::of(&WorkerContent { cube_id: cube.cube_id, policy_id });
        Self {
            worker_id,
            cube_id: cube.cube_id,
            status: WorkerStatus::Registered,
            policy_id,
        }
    }

    pub fn mark_processed(mut self, yield_count: u64) -> Self {
        self.status = WorkerStatus::Processed { yield_count };
        self
    }
}

/// Serialize-only for CubeMandorla content-addressing.
#[derive(Serialize)]
struct MandorlaContent {
    cube_ids: Vec<Digest>,
    overlap_score: i64,
    policy_id: Digest,
}

/// The structural overlap between source cubes targeting the same void.
///
/// Named after the mandorla (vesica piscis intersection region). When ≥2
/// source cubes address the same void they form a mandorla.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CubeMandorla {
    pub mandorla_id: Digest,
    /// Sorted cube IDs — order-independent.
    pub cube_ids: Vec<Digest>,
    /// Overlap score in [0, 1] as Q16 (integer arithmetic, no floats).
    pub overlap_score: Q16,
    pub policy_id: Digest,
}

impl CubeMandorla {
    pub fn from_cubes(mut cube_ids: Vec<Digest>, overlap_score: Q16, policy_id: Digest) -> Self {
        cube_ids.sort();
        let mandorla_id = Digest::of(&MandorlaContent {
            cube_ids: cube_ids.clone(),
            overlap_score: overlap_score.raw(),
            policy_id,
        });
        Self { mandorla_id, cube_ids, overlap_score, policy_id }
    }
}

/// Serialize-only for CompositeSupportCube content-addressing.
#[derive(Serialize)]
struct CompositeContent {
    source_cube_ids: Vec<Digest>,
    aggregate_support: i64,
    mandorla_ids: Vec<Digest>,
    policy_id: Digest,
}

/// Merged result of multiple SourceCubes processed by a CubeSwarm.
///
/// Content-addressed over the sorted set of source cube IDs; deterministic
/// regardless of the order cubes were registered in the swarm.
/// `aggregate_support` is the integer-averaged Q16 support score.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompositeSupportCube {
    pub composite_id: Digest,
    /// Sorted source cube IDs.
    pub source_cube_ids: Vec<Digest>,
    /// Integer-averaged Q16 support across constituent cubes (no floats).
    pub aggregate_support: Q16,
    pub mandorla_ids: Vec<Digest>,
    pub policy_id: Digest,
}

impl CompositeSupportCube {
    pub fn from_cubes(
        cubes: &[SourceCube],
        mandorlas: Vec<CubeMandorla>,
        policy_id: Digest,
    ) -> Self {
        let mut source_cube_ids: Vec<Digest> = cubes.iter().map(|c| c.cube_id).collect();
        source_cube_ids.sort();

        let aggregate_support = if cubes.is_empty() {
            Q16::ZERO
        } else {
            let sum_raw: i64 = cubes.iter().map(|c| c.support_score.raw()).sum();
            Q16::from_raw(sum_raw / cubes.len() as i64)
        };

        let mut mandorla_ids: Vec<Digest> = mandorlas.iter().map(|m| m.mandorla_id).collect();
        mandorla_ids.sort();

        let composite_id = Digest::of(&CompositeContent {
            source_cube_ids: source_cube_ids.clone(),
            aggregate_support: aggregate_support.raw(),
            mandorla_ids: mandorla_ids.clone(),
            policy_id,
        });

        Self {
            composite_id,
            source_cube_ids,
            aggregate_support,
            mandorla_ids,
            policy_id,
        }
    }
}

/// Coordinates multiple SourceCubeWorkers over a swarm of SourceCubes.
///
/// Cubes are sorted by cube_id at construction for deterministic ordering
/// (CROSS-014: replay stability). Processing is passive — no host mutations.
pub struct CubeSwarm {
    pub policy: PolicyProfile,
    pub source_cubes: Vec<SourceCube>,
}

impl CubeSwarm {
    pub fn new(policy: PolicyProfile, mut source_cubes: Vec<SourceCube>) -> Self {
        source_cubes.sort_by_key(|c| c.cube_id);
        Self { policy, source_cubes }
    }

    /// Run passively: register workers, detect mandorlas, build composite.
    pub fn run(&self) -> (Vec<SourceCubeWorker>, CompositeSupportCube) {
        let workers: Vec<SourceCubeWorker> = self
            .source_cubes
            .iter()
            .map(|cube| SourceCubeWorker::for_cube(cube, self.policy.id))
            .collect();

        let mandorlas = self.detect_mandorlas();
        let composite =
            CompositeSupportCube::from_cubes(&self.source_cubes, mandorlas, self.policy.id);

        (workers, composite)
    }

    /// Detect mandorlas: groups of ≥2 cubes targeting the same void.
    fn detect_mandorlas(&self) -> Vec<CubeMandorla> {
        let mut void_to_cubes: BTreeMap<Digest, Vec<Digest>> = BTreeMap::new();

        for cube in &self.source_cubes {
            if let Some(void_id) = cube.target_void_id {
                void_to_cubes.entry(void_id).or_default().push(cube.cube_id);
            }
        }

        let mut mandorlas: Vec<CubeMandorla> = void_to_cubes
            .into_values()
            .filter(|ids| ids.len() > 1)
            .map(|ids| {
                let n = ids.len() as u64;
                let overlap = Q16::ratio(1, n).unwrap_or(Q16::ZERO);
                CubeMandorla::from_cubes(ids, overlap, self.policy.id)
            })
            .collect();

        mandorlas.sort_by_key(|m| m.mandorla_id);
        mandorlas
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube::{CubeDimensionProfile, SourceCube};
    use kosmo_core::{Digest, PolicyProfile, Q16, TaintLabel};

    fn make_cube(void_id: Digest, path: &str, policy_id: Digest) -> SourceCube {
        SourceCube::new(
            Some(void_id),
            path.into(),
            CubeDimensionProfile::empty(),
            Q16::HALF,
            TaintLabel::Synthetic,
            Digest::ZERO,
            policy_id,
        )
    }

    #[test]
    fn cube_mandorla_is_order_independent() {
        let pid = Digest::of_bytes(b"p");
        let id1 = Digest::of_bytes(b"a");
        let id2 = Digest::of_bytes(b"b");
        let m1 = CubeMandorla::from_cubes(vec![id1, id2], Q16::HALF, pid);
        let m2 = CubeMandorla::from_cubes(vec![id2, id1], Q16::HALF, pid);
        assert_eq!(m1.mandorla_id, m2.mandorla_id);
    }

    #[test]
    fn fixture_source_cubes_merge_deterministically() {
        let policy = PolicyProfile::default_report_only();
        let void_id = Digest::of_bytes(b"v");
        let cubes = vec![
            make_cube(void_id, "src/a.rs", policy.id),
            make_cube(void_id, "src/b.rs", policy.id),
        ];
        // Swarm 1: insertion order a, b
        let swarm1 = CubeSwarm::new(policy.clone(), cubes.clone());
        // Swarm 2: insertion order b, a (reversed)
        let swarm2 = CubeSwarm::new(policy.clone(), {
            let mut rev = cubes.clone();
            rev.reverse();
            rev
        });
        let (_, c1) = swarm1.run();
        let (_, c2) = swarm2.run();
        assert_eq!(c1.composite_id, c2.composite_id, "fixture SourceCubes must merge deterministically");
    }

    #[test]
    fn swarm_detects_mandorla_for_shared_void() {
        let policy = PolicyProfile::default_report_only();
        let void_id = Digest::of_bytes(b"void");
        let cubes = vec![
            make_cube(void_id, "src/a.rs", policy.id),
            make_cube(void_id, "src/b.rs", policy.id),
        ];
        let swarm = CubeSwarm::new(policy, cubes);
        let (_, composite) = swarm.run();
        assert_eq!(composite.source_cube_ids.len(), 2);
        assert!(!composite.mandorla_ids.is_empty(), "shared void must produce a mandorla");
    }

    #[test]
    fn swarm_aggregate_support_is_integer_average() {
        let policy = PolicyProfile::default_report_only();
        let void_id = Digest::of_bytes(b"v");
        let cubes = vec![
            make_cube(void_id, "src/a.rs", policy.id),
            make_cube(void_id, "src/b.rs", policy.id),
        ];
        let swarm = CubeSwarm::new(policy, cubes);
        let (_, composite) = swarm.run();
        // Both cubes have HALF support → avg(HALF, HALF) = HALF
        assert_eq!(composite.aggregate_support, Q16::HALF);
    }

    #[test]
    fn swarm_empty_produces_zero_support() {
        let policy = PolicyProfile::default_report_only();
        let swarm = CubeSwarm::new(policy, vec![]);
        let (workers, composite) = swarm.run();
        assert!(workers.is_empty());
        assert_eq!(composite.aggregate_support, Q16::ZERO);
        assert_ne!(composite.composite_id, Digest::ZERO);
    }

    #[test]
    fn worker_is_content_addressed() {
        let policy = PolicyProfile::default_report_only();
        let void_id = Digest::of_bytes(b"v");
        let cube = make_cube(void_id, "src/foo.rs", policy.id);
        let w1 = SourceCubeWorker::for_cube(&cube, policy.id);
        let w2 = SourceCubeWorker::for_cube(&cube, policy.id);
        assert_eq!(w1.worker_id, w2.worker_id);
        assert_ne!(w1.worker_id, Digest::ZERO);
    }
}

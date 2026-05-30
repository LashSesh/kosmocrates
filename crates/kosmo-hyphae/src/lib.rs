//! Kosmocrates HYPHAE v0.3/v0.4 — run-local topology assimilation.
//!
//! Phase 3 passive run: workspace scan → void map → deficiency vector →
//! frontier graph → gate cascade → assimilation decisions.
//!
//! Phase 4 CubeSwarm MVP: source cubes → swarm workers → composite support
//! cube → host target delta (planning only).
//!
//! No host file writes. All outputs are observations and decisions only.
//! Default mode: `PolicyProfile::default_report_only()`.

pub mod assimilation;
pub mod code_hdag;
pub mod cube;
pub mod deficiency;
pub mod delta;
pub mod frontier;
pub mod gates;
pub mod host;
pub mod motif;
pub mod run;
pub mod structural_yield;
pub mod swarm;
pub mod void_map;

pub use assimilation::{AssimilationDecision, AssimilationOutcome, NegativeEvidenceRecord};
pub use code_hdag::{CodeHDAG, CodeObservation, HDAGEdge, HDAGEdgeKind, HDAGNode, ObservationKind};
pub use cube::{CubeDimensionProfile, RepositoryCube, SourceCube};
pub use deficiency::{DeficiencyEntry, DeficiencyKind, DeficiencyVector};
pub use delta::{DeltaAction, DeltaStatus, HostTargetDelta, VoidFillDelta};
pub use frontier::{SourceEvidence, SourceFrontierGraph, SourceIntent, SourceIntentKind};
pub use gates::{GateCascade, GateCheckRecord, GateKind, GateTrace};
pub use host::{HostBinding, HostCube};
pub use motif::MotifCandidate;
pub use run::{passive_run, HyphaeRunResult};
pub use structural_yield::{StructuralYield, StructuralYieldKind};
pub use swarm::{CompositeSupportCube, CubeMandorla, CubeSwarm, SourceCubeWorker, WorkerStatus};
pub use void_map::{HostVoid, HostVoidKind, TopologicalVoidMap};

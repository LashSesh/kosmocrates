//! Kosmocrates HYPHAE v0.3 — run-local topology assimilation.
//!
//! Phase 3 passive run: workspace scan → void map → deficiency vector →
//! frontier graph → gate cascade → assimilation decisions.
//!
//! No host file writes. All outputs are observations and decisions only.
//! Default mode: `PolicyProfile::default_report_only()`.

pub mod assimilation;
pub mod code_hdag;
pub mod deficiency;
pub mod frontier;
pub mod gates;
pub mod host;
pub mod motif;
pub mod run;
pub mod structural_yield;
pub mod void_map;

pub use assimilation::{AssimilationDecision, AssimilationOutcome, NegativeEvidenceRecord};
pub use code_hdag::{CodeHDAG, CodeObservation, HDAGEdge, HDAGEdgeKind, HDAGNode, ObservationKind};
pub use deficiency::{DeficiencyEntry, DeficiencyKind, DeficiencyVector};
pub use frontier::{SourceEvidence, SourceFrontierGraph, SourceIntent, SourceIntentKind};
pub use gates::{GateCascade, GateCheckRecord, GateKind, GateTrace};
pub use host::{HostBinding, HostCube};
pub use motif::MotifCandidate;
pub use run::{passive_run, HyphaeRunResult};
pub use structural_yield::{StructuralYield, StructuralYieldKind};
pub use void_map::{HostVoid, HostVoidKind, TopologicalVoidMap};

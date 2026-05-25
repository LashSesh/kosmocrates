//! # PSE — Post-Symbolic Engine
//!
//! A universal computation engine that processes information through
//! topology, physics, and geometry rather than through symbols or statistics.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use pse::prelude::*;
//!
//! // Implement ObservationAdapter for your domain, then:
//! // let config = Config::default();
//! // let mut state = GlobalState::new(&config);
//! // let adapter = PassthroughAdapter::new("my-domain");
//! // let result = macro_step(&mut state, &observations, &config, &adapter);
//! ```

/// AES-256-GCM capsule encryption with policy-gated seal/open.
pub use pse_capsule as capsule;
/// Adversarial validation cascade: MetricSet, dual_consensus, PoRFsm.
pub use pse_cascade as cascade;
/// Constraint propagation: morphogenic mutations, DoF analysis.
pub use pse_constraint as constraint;
/// Engine orchestrator: GlobalState, macro_step, DomainAdapter trait.
pub use pse_core as core;
/// Evidence chain construction and crystal verification.
pub use pse_evidence as evidence;
/// Pattern extraction: inverse_weave, operator library.
pub use pse_extract as extract;
/// Observation graph: ObservationAdapter, PassthroughAdapter, PersistentGraph.
pub use pse_graph as graph;
/// Execution manifest construction and verification.
pub use pse_manifest as manifest;
/// TRITON navigator: golden-angle spiral, spectral-guided exploration.
pub use pse_navigator as navigator;
/// Polycentric multi-hypothesis drill engine.
pub use pse_pmhd as pmhd;
/// Digest-bound operator registry.
pub use pse_registry as registry;
/// Deterministic replay and verification.
pub use pse_replay as replay;
/// Multi-scale observation: micro/meso/macro universes, Kuramoto synchronization.
pub use pse_scale as scale;
/// Tick-based adaptive scheduling.
pub use pse_scheduler as scheduler;
/// SQLite persistence layer.
pub use pse_store as store;
/// Multi-agent swarm coordination with deterministic consensus.
pub use pse_swarm as swarm;
/// Topological analysis: Laplacian, Fiedler, Betti, Kuramoto, CTQW, DTL.
pub use pse_topology as topology;
/// Core types: SemanticCrystal, Observation, VertexId, Hash256, EvidenceChain, Config.
pub use pse_types as types;
/// IL adapter: ILStore, text_to_vector8 — char-4-gram HDAG knowledge store.
pub use pse_adapter_il as il;
/// ETV reasoning: guide (single-path) and guide_beam (Quantum-Walk) over ILStore.
pub use pse_reasoning as reasoning;

/// Prelude — import everything you need for a typical PSE application.
pub mod prelude {
    pub use pse_core::*;
    pub use pse_types::*;
}

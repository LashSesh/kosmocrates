//! Attractor-constrained rule evolution for nxalien.
//!
//! Closes the PSE → nxalien feedback loop:
//!
//!   nxalien bundle → PSE Observation → PersistentGraph → attractor centroid
//!         ↓                                                      ↓
//!   next compile ← validated rule proposals ← EpistemicSignal ←┘
//!
//! Rules evolve toward the free-energy minimum of the knowledge field —
//! not arbitrarily, but constrained by PSE's own attractor dynamics.
//! An EvolutionGuard prevents unbounded drift by requiring attractor
//! alignment above a configurable threshold.

pub mod evolution;
pub mod graph_state;
pub mod il_bridge;
pub mod signal;

pub use evolution::{apply_validated_proposals, propose_rule_evolution, EvolutionGuard};
pub use graph_state::{GraphState, NXALIEN_SOURCE_ID};
pub use il_bridge::{
    agenda_to_unknowns, commit_rules_to_il, load_il_health_and_agenda,
    AgendaItemSnapshot, ILCommitSummary, ILHealthSnapshot,
};
pub use signal::{EpistemicSignal, SignalStability};

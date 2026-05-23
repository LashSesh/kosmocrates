//! PSE → Infinity Ledger bridge.
//!
//! Converts `SemanticCrystal` + provenance into IL-compatible artifacts:
//! - `ILPayload`: a JSON representation of a `CompactTic` ledger block
//! - An 8D vector: [fixpoint[0..5], ψ, ρ, ω] normalised to unit length
//!
//! `ILStore` persists those blocks on disk in IL ledger format, provides
//! cosine-similarity search, and maintains an HDAG of all committed crystals.
//!
//! ## HDAG
//! The HDAG is always active (no feature flag required).  It implements
//! Sebastian Klemm's original HDAG v1.0 specification:
//! - Nodes are 5D resonance tensors [temporal, morphic, relational, topological, entropic]
//! - Edges are phase-gradient transitions Φ_ij = (T_j − T_i) / ‖T_j − T_i‖
//! - Acyclicity emerges from the coherence potential gate, not explicit checks
//!
//! ## Features
//! - `il-pipeline`: drive MEFCore::process() for authoritative TIC generation
//! - `full`:        il-pipeline enabled

pub mod adapter;
pub mod hdag;
pub mod store;

pub use adapter::{text_to_vector8, CrystalAdapter, ILPayload};
pub use hdag::{crystal_to_tensor, HDAGEdge, PathInvarianceResult, ResonanceTensor, HDAG};
pub use store::{ILMatch, ILStore};

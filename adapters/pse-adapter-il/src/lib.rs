//! PSE → Infinity Ledger bridge.
//!
//! Converts `SemanticCrystal` + provenance into IL-compatible artifacts:
//! - `ILPayload`: a JSON representation of a `CompactTic` ledger block
//! - An 8D vector: [fixpoint[0..5], ψ, ρ, ω] normalised to unit length
//!
//! `ILStore` persists those blocks on disk in IL ledger format and provides
//! cosine-similarity search over the accumulated 8D vectors — no running IL
//! server required.
//!
//! ## Features
//! - `hdag`:        wire every commit into an IL HDAG temporal-causal graph
//! - `il-pipeline`: drive MEFCore::process() for authoritative TIC generation
//! - `full`:        all of the above

pub mod adapter;
pub mod store;

pub use adapter::{text_to_vector8, CrystalAdapter, ILPayload};
pub use store::{ILMatch, ILStore};

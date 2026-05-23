//! PSE → Infinity Ledger bridge.
//!
//! Converts `SemanticCrystal` + `CrystalRecord` into IL-compatible artifacts:
//! - `ILBlock`: a JSON representation of a `CompactTic` ledger block
//! - An 8D HNSW vector: [fixpoint[0..5], ψ, ρ, ω] normalised to unit length
//!
//! `ILStore` persists those blocks on disk in IL ledger format and provides
//! cosine-similarity search over the accumulated 8D vectors — no running IL
//! server required.  When an IL API server is available, call `ILStore::api_url`
//! instead.

pub mod adapter;
pub mod store;

pub use adapter::{CrystalAdapter, ILPayload};
pub use store::{ILMatch, ILStore};

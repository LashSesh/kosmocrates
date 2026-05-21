//! LPCM evidence and input window types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::primitives::{EvidenceRef, Hash256, StableId};

/// A bounded evidence window passed as input to the LPCM pipeline.
/// The window is immutable after canonicalization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpcmInputWindow {
    /// Stable identifier for this window.
    pub window_id: StableId,
    /// SHA-256 of the canonical serialization of the raw window bytes.
    pub window_hash: Hash256,
    /// Ordered list of evidence refs in this window.
    pub evidence_refs: Vec<EvidenceRef>,
    /// Ordered adjacency map: evidence hash -> set of adjacent evidence hashes.
    pub adjacency: BTreeMap<Hash256, Vec<Hash256>>,
    /// Optional carrier hash from upstream topology.
    pub carrier_hash: Option<Hash256>,
    /// Optional topology hash from upstream TPT/MTL.
    pub source_topology_hash: Option<Hash256>,
}

//! Fragmented phase-space window and fragment reference types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::primitives::{EvidenceRef, Hash256};

/// The result of partitioning an input window into deterministic fragments.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentedPhaseSpaceWindow {
    /// Content-addressed id of this fragmented window (hash of canonical form).
    pub window_id: Hash256,
    /// Hash of the source input window.
    pub source_window_hash: Hash256,
    /// Ordered list of fragment references (sorted by ordinal).
    pub fragments: Vec<FragmentRef>,
    /// Fragment adjacency map: fragment_id -> set of adjacent fragment_ids.
    pub adjacency: BTreeMap<Hash256, Vec<Hash256>>,
    /// Content address of the partition report (reproducibility reference).
    pub partition_report_ref: Hash256,
}

/// Reference to a single fragment within a partitioned window.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FragmentRef {
    /// Content address of this fragment's canonical representation.
    pub fragment_id: Hash256,
    /// Sequential ordinal within the parent window (determines processing order).
    pub ordinal: u64,
    /// Evidence refs bound to this fragment.
    pub evidence_refs: Vec<EvidenceRef>,
    /// Canonical hash of the boundary between this fragment and adjacent ones.
    pub boundary_hash: Hash256,
    /// Optional carrier hash inherited from upstream.
    pub carrier_hash: Option<Hash256>,
}

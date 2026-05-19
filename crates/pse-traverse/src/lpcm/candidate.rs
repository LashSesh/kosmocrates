//! Candidate direction types for LPCM.

use serde::{Deserialize, Serialize};

use super::primitives::{EvidenceRef, Hash256};

/// A single candidate direction generated for a fragment.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CandidateDirection {
    /// Content address of this candidate.
    pub candidate_id: Hash256,
    /// Fragment this candidate belongs to.
    pub fragment_id: Hash256,
    /// What kind of local operation this candidate proposes.
    pub direction_kind: CandidateDirectionKind,
    /// Canonical hash of the proposed direction's signature.
    pub direction_signature_hash: Hash256,
    /// Canonical hash of the proposed boundary delta.
    pub proposed_boundary_delta_hash: Hash256,
    /// Canonical hash of the proposed seam delta.
    pub proposed_seam_delta_hash: Hash256,
    /// Evidence refs supporting this candidate.
    pub evidence_refs: Vec<EvidenceRef>,
}

/// The kind of local operation this candidate proposes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CandidateDirectionKind {
    CompleteGap,
    ContinuePath,
    ContractRegion,
    RefineMesh,
    BridgeBoundary,
    RemoveContradiction,
    HoldDiagnostic,
}

//! Fabric-T: `FieldTensorState` and `CouplingMatrix`
//! (PHASEMATRIX-HIVEMIND-03.1 §6).
//!
//! Fabric-T is the **persistent** coupling structure of the substrate.
//! It may only be mutated through an accepted [`super::coupling_update::CouplingUpdate`];
//! Fabric-H events MUST NOT touch it directly (Invariant 1).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, PhaseError};

/// Coupling kind (§6.1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CouplingKind {
    /// Structural topology link.
    Structural,
    /// Resonance-derived link.
    Resonance,
    /// Temporal correlation link.
    Temporal,
    /// Semantic similarity link.
    Semantic,
    /// Matrix-boundary seam link.
    Boundary,
}

/// One directed weighted edge in the coupling matrix (§6.1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CouplingEntry {
    /// Source node / cell hash.
    pub source: Hash256,
    /// Target node / cell hash.
    pub target: Hash256,
    /// Float-free coupling weight.
    pub weight: Fixed,
    /// Semantic category of this coupling.
    pub coupling_kind: CouplingKind,
    /// Content address of the last `CouplingUpdate` that touched this entry.
    pub last_update_hash: Option<Hash256>,
}

/// Content-addressed coupling matrix (§6.1).
///
/// Entries MUST be sorted (by `(source, target, coupling_kind)`) before
/// hashing to guarantee determinism.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CouplingMatrix {
    /// Content address of this matrix.
    pub matrix_id: Hash256,
    /// Sorted coupling entries.
    pub entries: Vec<CouplingEntry>,
}

impl CouplingMatrix {
    /// Compute `matrix_id` from a sorted entry list.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.entries.sort();
        let mut probe = self.clone();
        probe.matrix_id = Hash256::zero();
        self.matrix_id = content_address(&probe)?;
        Ok(self)
    }

    /// Empty coupling matrix (initial state).
    pub fn empty() -> Result<Self, PhaseError> {
        CouplingMatrix {
            matrix_id: Hash256::zero(),
            entries: Vec::new(),
        }
        .with_id()
    }
}

/// Persistent field-tensor state (§6.1 Fabric-T).
///
/// Only `CouplingUpdate`s accepted by the `StitcherGate` may create a
/// new revision of this state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FieldTensorState {
    /// Content address of this tensor snapshot.
    pub tensor_id: Hash256,
    /// Monotonically increasing revision counter.
    pub tensor_revision: u64,
    /// Anchoring matrix id (mirrors `PhaseMatrixRunDescriptorV3::matrix_id`).
    pub matrix_id: Hash256,
    /// Content addresses of all active node state artefacts.
    pub node_state_hashes: Vec<Hash256>,
    /// Content address of the spatial topology (FunnelGraph or equivalent).
    pub spatial_topology_hash: Hash256,
    /// Content address of the current `CouplingMatrix`.
    pub coupling_matrix_hash: Hash256,
    /// Content addresses of all active seam-state artefacts.
    pub seam_state_hashes: Vec<Hash256>,
    /// Content address of the preceding `FieldTensorState` revision (chain).
    pub previous_tensor_hash: Option<Hash256>,
    /// Head trace hash linking to the latest `FieldTensorTrace`.
    pub trace_head: Hash256,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl FieldTensorState {
    /// Recompute `tensor_id` from the current field values.
    ///
    /// Sorts `node_state_hashes` and `seam_state_hashes` before hashing.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.node_state_hashes.sort();
        self.seam_state_hashes.sort();
        let mut probe = self.clone();
        probe.tensor_id = Hash256::zero();
        self.tensor_id = content_address(&probe)?;
        Ok(self)
    }

    /// Create the genesis tensor for a given matrix.
    pub fn genesis(matrix_id: Hash256, coupling_matrix_hash: Hash256) -> Result<Self, PhaseError> {
        let spatial_topology_hash = content_address(&("genesis.spatial", &matrix_id))?;
        FieldTensorState {
            tensor_id: Hash256::zero(),
            tensor_revision: 0,
            matrix_id,
            node_state_hashes: Vec::new(),
            spatial_topology_hash,
            coupling_matrix_hash,
            seam_state_hashes: Vec::new(),
            previous_tensor_hash: None,
            trace_head: Hash256::zero(),
            evidence_refs: Vec::new(),
        }
        .with_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_tensor_is_content_addressed() {
        let m = Hash256::zero();
        let cm = CouplingMatrix::empty().unwrap();
        let t = FieldTensorState::genesis(m.clone(), cm.matrix_id.clone()).unwrap();
        let t2 = FieldTensorState::genesis(m, cm.matrix_id).unwrap();
        assert_eq!(t.tensor_id, t2.tensor_id);
    }

    #[test]
    fn coupling_matrix_entries_are_sorted_before_hashing() {
        let w = Fixed::Rational { num: 1, den: 2 };
        let a = Hash256([1u8; 32]);
        let b = Hash256([2u8; 32]);
        let e1 = CouplingEntry {
            source: a.clone(),
            target: b.clone(),
            weight: w.clone(),
            coupling_kind: CouplingKind::Resonance,
            last_update_hash: None,
        };
        let e2 = CouplingEntry {
            source: b.clone(),
            target: a.clone(),
            weight: w.clone(),
            coupling_kind: CouplingKind::Structural,
            last_update_hash: None,
        };
        let m1 = CouplingMatrix {
            matrix_id: Hash256::zero(),
            entries: vec![e1.clone(), e2.clone()],
        }
        .with_id()
        .unwrap();
        let m2 = CouplingMatrix {
            matrix_id: Hash256::zero(),
            entries: vec![e2, e1],
        }
        .with_id()
        .unwrap();
        assert_eq!(m1.matrix_id, m2.matrix_id, "sort before hash must commute");
    }
}

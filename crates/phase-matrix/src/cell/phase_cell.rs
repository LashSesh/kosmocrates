//! `PhaseCell` (§8.2).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, PhaseError};
use super::trident_vector::TridentVector;

/// Cell role per §8.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PhaseCellRole {
    /// Sense external observations.
    Sensor,
    /// Carry resonance amplitude.
    Resonator,
    /// Route information between cells.
    Router,
    /// Validate gate-relevant evidence.
    Validator,
    /// Probe the spiral memory layer.
    MemoryProbe,
    /// Guard the matrix boundary.
    BoundaryGuard,
    /// Emit candidate intents.
    CandidateEmitter,
    /// Regulate morphology decisions.
    MorphologyRegulator,
}

/// Cell lifecycle per §8.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CellLifecycle {
    /// Cell exists but is not contributing.
    Dormant,
    /// Cell is producing pulses.
    Active,
    /// Cell is part of a cluster.
    Clustered,
    /// Cell is decaying.
    Decaying,
    /// Cell has been compacted into a trace.
    Compacted,
    /// Cell is retired and may not be referenced again.
    Retired,
}

/// `PhaseCell` (§8.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseCell {
    /// Content-addressed cell id.
    pub cell_id: Hash256,
    /// Parent node id.
    pub parent_node_id: Hash256,
    /// Trident vector.
    pub trident: TridentVector,
    /// Local resonance amplitude.
    pub local_resonance: Fixed,
    /// Harmonisation score.
    pub harmonization: Fixed,
    /// Optional spiral-memory state hash.
    pub spiral_memory_state: Option<Hash256>,
    /// Cell role.
    pub role: PhaseCellRole,
    /// Cell lifecycle stage.
    pub lifecycle: CellLifecycle,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl PhaseCell {
    /// Recompute the content-addressed `cell_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.cell_id = Hash256::zero();
        self.cell_id = content_address(&probe)?;
        Ok(self)
    }

    /// Build a synthetic cell deterministically from `(parent, index)`.
    pub fn synthetic(
        parent_node_id: Hash256,
        index: u32,
        role: PhaseCellRole,
    ) -> Result<Self, PhaseError> {
        let trident = TridentVector {
            semantic_density: Fixed::Rational {
                num: 1 + (index as i128 % 5),
                den: 10,
            },
            structural_coherence: Fixed::Rational {
                num: 2 + (index as i128 % 4),
                den: 10,
            },
            temporal_phase: Fixed::Rational {
                num: 3 + (index as i128 % 3),
                den: 10,
            },
        };
        let cell = PhaseCell {
            cell_id: Hash256::zero(),
            parent_node_id,
            trident,
            local_resonance: Fixed::Rational {
                num: 5 + (index as i128 % 3),
                den: 10,
            },
            harmonization: Fixed::Rational {
                num: 5 + (index as i128 % 5),
                den: 10,
            },
            spiral_memory_state: None,
            role,
            lifecycle: CellLifecycle::Active,
            evidence_refs: Vec::new(),
        };
        cell.with_id()
    }
}

impl PartialOrd for PhaseCell {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PhaseCell {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Order by `cell_id` so the canonical encoding of a `CellPool`
        // is permutation-invariant.
        self.cell_id.cmp(&other.cell_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_cell_hash_is_stable() {
        let a = PhaseCell::synthetic(Hash256::zero(), 0, PhaseCellRole::Resonator).unwrap();
        let b = PhaseCell::synthetic(Hash256::zero(), 0, PhaseCellRole::Resonator).unwrap();
        assert_eq!(a.cell_id, b.cell_id);
    }

    #[test]
    fn phase_cell_id_changes_with_role() {
        let a = PhaseCell::synthetic(Hash256::zero(), 0, PhaseCellRole::Resonator).unwrap();
        let b = PhaseCell::synthetic(Hash256::zero(), 0, PhaseCellRole::Validator).unwrap();
        assert_ne!(a.cell_id, b.cell_id);
    }
}

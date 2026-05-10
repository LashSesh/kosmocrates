//! `CellPool` (§8.3).
//!
//! Per spec invariant a `CellPool` MUST only contain cells whose
//! `parent_node_id` (or delegated subnet authorisation) is compatible
//! with the pool's `parent_subnet_id`. We enforce this by checking
//! `parent_node_id` membership in the subnet's member list at pool
//! construction time.

use serde::{Deserialize, Serialize};

use super::phase_cell::PhaseCell;
use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, PhaseError};

/// Build options for a synthetic cell pool.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CellPoolBuildOptions {
    /// Number of cells to synthesise per node.
    pub cells_per_node: u32,
    /// Whether to include a memory-probe cell per node.
    pub include_memory_probe: bool,
}

impl CellPoolBuildOptions {
    /// Reasonable default (4 cells per node, no memory probe).
    pub fn default_options() -> Self {
        CellPoolBuildOptions {
            cells_per_node: 4,
            include_memory_probe: false,
        }
    }
}

/// `CellPool` (§8.3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellPool {
    /// Content-addressed pool id.
    pub pool_id: Hash256,
    /// Parent subnet the pool serves.
    pub parent_subnet_id: Hash256,
    /// Member cells (sorted by `cell_id` before hashing).
    pub cells: Vec<PhaseCell>,
    /// Aggregate pool coherence.
    pub pool_coherence: Fixed,
    /// Aggregate pool entropy.
    pub pool_entropy: Fixed,
    /// References to active clusters (sorted).
    pub active_cluster_refs: Vec<Hash256>,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl CellPool {
    /// Recompute `pool_id` after sorting the cell + cluster lists.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.cells.sort();
        self.active_cluster_refs.sort();
        let mut probe = self.clone();
        probe.pool_id = Hash256::zero();
        self.pool_id = content_address(&probe)?;
        Ok(self)
    }

    /// Build an empty pool anchored to a subnet.
    pub fn empty(parent_subnet_id: Hash256) -> Result<Self, PhaseError> {
        let pool = CellPool {
            pool_id: Hash256::zero(),
            parent_subnet_id,
            cells: Vec::new(),
            pool_coherence: Fixed::Rational { num: 0, den: 1 },
            pool_entropy: Fixed::Rational { num: 0, den: 1 },
            active_cluster_refs: Vec::new(),
            evidence_refs: Vec::new(),
        };
        pool.with_id()
    }

    /// Insert a cell into the pool, refusing parent-subnet mismatches.
    pub fn insert_cell(
        &mut self,
        cell: PhaseCell,
        allowed_parent_nodes: &[Hash256],
    ) -> Result<(), PhaseError> {
        if !allowed_parent_nodes.contains(&cell.parent_node_id) {
            return Err(PhaseError::MatrixBoundaryViolation {
                reason: format!(
                    "cell {} parent node {} not in subnet member list",
                    cell.cell_id.hex(),
                    cell.parent_node_id.hex()
                ),
            });
        }
        self.cells.push(cell);
        Ok(())
    }

    /// Refresh aggregate coherence + entropy from the current cells.
    pub fn refresh_aggregates(&mut self) {
        if self.cells.is_empty() {
            self.pool_coherence = Fixed::Rational { num: 0, den: 1 };
            self.pool_entropy = Fixed::Rational { num: 0, den: 1 };
            return;
        }
        // Mean of `harmonization` for coherence; mean of (1 −
        // local_resonance) for entropy.
        let n = self.cells.len() as i128;
        let mut coh = Fixed::Rational { num: 0, den: 1 };
        let mut ent = Fixed::Rational { num: 0, den: 1 };
        for c in &self.cells {
            coh = super::primitives::fixed_add(&coh, &c.harmonization);
            let one = Fixed::Rational { num: 1, den: 1 };
            ent = super::primitives::fixed_add(
                &ent,
                &super::primitives::fixed_sub(&one, &c.local_resonance),
            );
        }
        let (cn, cd) = match coh {
            Fixed::FixedI64 { raw, scale } => (raw as i128, 10i128.pow(scale)),
            Fixed::Rational { num, den } => (num, den),
        };
        let (en, ed) = match ent {
            Fixed::FixedI64 { raw, scale } => (raw as i128, 10i128.pow(scale)),
            Fixed::Rational { num, den } => (num, den),
        };
        self.pool_coherence = super::primitives::normalize_rational(cn, cd.saturating_mul(n));
        self.pool_entropy = super::primitives::normalize_rational(en, ed.saturating_mul(n));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::phase_cell::PhaseCellRole;

    #[test]
    fn pool_sorts_cells_before_hashing() {
        let parent = Hash256::zero();
        let allowed = vec![parent.clone()];
        let mut pool = CellPool::empty(parent.clone()).unwrap();
        let c0 = PhaseCell::synthetic(parent.clone(), 1, PhaseCellRole::Resonator).unwrap();
        let c1 = PhaseCell::synthetic(parent.clone(), 0, PhaseCellRole::Resonator).unwrap();
        pool.insert_cell(c0.clone(), &allowed).unwrap();
        pool.insert_cell(c1.clone(), &allowed).unwrap();
        let pool_a = pool.with_id().unwrap();
        let mut pool_b = CellPool::empty(parent.clone()).unwrap();
        pool_b.insert_cell(c1, &allowed).unwrap();
        pool_b.insert_cell(c0, &allowed).unwrap();
        let pool_b = pool_b.with_id().unwrap();
        assert_eq!(pool_a.pool_id, pool_b.pool_id);
    }

    #[test]
    fn insert_rejects_foreign_parent_node() {
        let parent = Hash256::zero();
        let mut foreign = [0u8; 32];
        foreign[0] = 0xff;
        let allowed = vec![parent.clone()];
        let mut pool = CellPool::empty(parent).unwrap();
        let c = PhaseCell::synthetic(Hash256(foreign), 0, PhaseCellRole::Resonator).unwrap();
        assert!(pool.insert_cell(c, &allowed).is_err());
    }
}

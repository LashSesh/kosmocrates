//! Layer C4 (cont.) — Hypercube Puzzle and Entropy Collapse Certificate
//! (§9.3, §9.4).
//!
//! The puzzle state is a structured snapshot of partial assignments,
//! candidate sets, hidden singles, boundary contracts and negative
//! topology witnesses. The `EntropyCollapseCertificate` is the
//! gate-effective proof that the puzzle reduced its entropy below the
//! declared threshold.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::constraint_lattice::ConstraintLatticeCognition;
use super::primitives::{fixed_ge, fixed_le, CognitionError, Fixed, StableId};
use super::run_descriptor::CognitionRunDescriptor;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CognitiveDimension {
    pub dimension_id: StableId,
    pub size: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PartialAssignment {
    pub dimension_id: StableId,
    pub value_hash: Hash256,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CandidateSet {
    pub dimension_id: StableId,
    pub candidates: Vec<Hash256>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HiddenSingle {
    pub dimension_id: StableId,
    pub value_hash: Hash256,
    pub support: Vec<Hash256>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BoundaryContract {
    pub contract_id: Hash256,
    pub from_dimension: StableId,
    pub to_dimension: StableId,
    pub guard_hash: Hash256,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NegativeTopologyWitness {
    pub witness_id: Hash256,
    pub forbidden_pattern_hash: Hash256,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntropyCollapseCertificate {
    pub certificate_id: Hash256,
    pub cube_id: Hash256,
    pub lattice_id: Hash256,
    pub entropy_before: Fixed,
    pub entropy_after: Fixed,
    pub weighted_constraint_mass: Fixed,
    pub feasible_set_uniqueness: Fixed,
    pub boundary_contract_hashes: Vec<Hash256>,
    pub issued_at_logical_step: u64,
}

impl EntropyCollapseCertificate {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        self.boundary_contract_hashes.sort();
        let mut probe = self.clone();
        probe.certificate_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.certificate_id = Hash256(id);
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypercubePuzzleState {
    pub cube_id: Hash256,
    pub dimensions: Vec<CognitiveDimension>,
    pub partial_assignments: Vec<PartialAssignment>,
    pub candidate_sets: Vec<CandidateSet>,
    pub hidden_singles: Vec<HiddenSingle>,
    pub boundary_contracts: Vec<BoundaryContract>,
    pub negative_topology: Vec<NegativeTopologyWitness>,
    pub collapse_certificate: Option<EntropyCollapseCertificate>,
}

impl HypercubePuzzleState {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        self.dimensions.sort();
        self.partial_assignments.sort();
        self.candidate_sets.sort();
        self.hidden_singles.sort();
        self.boundary_contracts.sort();
        self.negative_topology.sort();
        let mut probe = self.clone();
        probe.cube_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.cube_id = Hash256(id);
        Ok(self)
    }

    /// Detect hidden singles deterministically: for each candidate set
    /// of size 1, emit a `HiddenSingle` referencing the lone candidate.
    /// Stable across permutations of the input candidate sets.
    pub fn detect_hidden_singles(&mut self) {
        let mut singles = Vec::new();
        for cs in &self.candidate_sets {
            if cs.candidates.len() == 1 {
                singles.push(HiddenSingle {
                    dimension_id: cs.dimension_id.clone(),
                    value_hash: cs.candidates[0].clone(),
                    support: vec![],
                });
            }
        }
        singles.sort();
        self.hidden_singles = singles;
    }
}

/// Build a minimal puzzle from the lattice and the descriptor's
/// hypercube dimension count. Issues an `EntropyCollapseCertificate`
/// when the lattice already exhibits a constraint mass + uniqueness
/// pair above the declared thresholds.
pub fn build_puzzle_minimal(
    lattice: &ConstraintLatticeCognition,
    rd: &CognitionRunDescriptor,
    logical_step: u64,
) -> Result<HypercubePuzzleState, CognitionError> {
    let n = rd.policies.hypercube_dimensions as usize;
    let mut dims = Vec::with_capacity(n);
    for i in 0..n {
        dims.push(CognitiveDimension {
            dimension_id: format!("d{i}"),
            size: 2,
        });
    }
    let entropy_before = Fixed::Rational {
        num: n as i128,
        den: 1,
    };
    let entropy_after = lattice.entropy_reduction.clone();
    let mut puzzle = HypercubePuzzleState {
        cube_id: Hash256::zero(),
        dimensions: dims,
        partial_assignments: Vec::new(),
        candidate_sets: Vec::new(),
        hidden_singles: Vec::new(),
        boundary_contracts: Vec::new(),
        negative_topology: Vec::new(),
        collapse_certificate: None,
    };
    puzzle.detect_hidden_singles();
    puzzle = puzzle.with_id()?;

    let collapse = if fixed_ge(
        &lattice.weighted_constraint_mass,
        &rd.thresholds.min_constraint_mass,
    ) && fixed_le(&entropy_after, &rd.thresholds.max_entropy)
    {
        Some(
            EntropyCollapseCertificate {
                certificate_id: Hash256::zero(),
                cube_id: puzzle.cube_id.clone(),
                lattice_id: lattice.lattice_id.clone(),
                entropy_before,
                entropy_after,
                weighted_constraint_mass: lattice.weighted_constraint_mass.clone(),
                feasible_set_uniqueness: lattice.feasible_set_uniqueness.clone(),
                boundary_contract_hashes: Vec::new(),
                issued_at_logical_step: logical_step,
            }
            .with_id()?,
        )
    } else {
        None
    };
    puzzle.collapse_certificate = collapse;
    puzzle.with_id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::constraint_lattice::build_lattice_minimal;
    use crate::cognition::run_descriptor::{CognitionPolicies, CognitionThresholds};
    use std::collections::BTreeMap;

    fn rd() -> CognitionRunDescriptor {
        CognitionRunDescriptor {
            run_id: "cognition.puzzle".into(),
            problem_spec_hash: Hash256::zero(),
            traversal_report_hash: None,
            projection_v2_hash: None,
            seed: 0,
            operator_versions: BTreeMap::new(),
            thresholds: CognitionThresholds::permissive(),
            policies: CognitionPolicies::default_policies(),
            canonicalization_version: "cognition-v0.1".into(),
        }
    }

    #[test]
    fn hidden_single_detector_is_stable() {
        let rd = rd();
        let lattice = build_lattice_minimal(&rd, 4, &Fixed::quantize(0.5, 9).unwrap()).unwrap();
        let mut puzzle_a = build_puzzle_minimal(&lattice, &rd, 0).unwrap();
        let mut puzzle_b = puzzle_a.clone();
        // Add the same candidate set in two orders.
        let one = {
            let mut x = [0u8; 32];
            x[0] = 1;
            Hash256(x)
        };
        let two = {
            let mut x = [0u8; 32];
            x[0] = 2;
            Hash256(x)
        };
        puzzle_a.candidate_sets = vec![
            CandidateSet {
                dimension_id: "d0".into(),
                candidates: vec![one.clone()],
            },
            CandidateSet {
                dimension_id: "d1".into(),
                candidates: vec![two.clone()],
            },
        ];
        puzzle_b.candidate_sets = vec![
            CandidateSet {
                dimension_id: "d1".into(),
                candidates: vec![two],
            },
            CandidateSet {
                dimension_id: "d0".into(),
                candidates: vec![one],
            },
        ];
        puzzle_a.detect_hidden_singles();
        puzzle_b.detect_hidden_singles();
        assert_eq!(puzzle_a.hidden_singles, puzzle_b.hidden_singles);
    }

    #[test]
    fn entropy_collapse_changes_on_boundary_change() {
        let rd = rd();
        let lattice = build_lattice_minimal(&rd, 4, &Fixed::quantize(0.5, 9).unwrap()).unwrap();
        let puzzle_a = build_puzzle_minimal(&lattice, &rd, 0).unwrap();
        let mut puzzle_b = puzzle_a.clone();
        let mut tampered = puzzle_b.collapse_certificate.clone().unwrap();
        let mut alt = [0u8; 32];
        alt[0] = 0xab;
        tampered.boundary_contract_hashes.push(Hash256(alt));
        let tampered = tampered.with_id().unwrap();
        puzzle_b.collapse_certificate = Some(tampered);
        let puzzle_b = puzzle_b.with_id().unwrap();
        assert_ne!(puzzle_a.cube_id, puzzle_b.cube_id);
    }
}

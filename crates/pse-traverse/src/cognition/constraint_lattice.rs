//! Layer C4 — Constraint Lattice and Hypercube Puzzle Collapse (§9).
//!
//! `ConstraintLatticeCognition` carries the weighted constraint mass,
//! entropy reduction and feasible-set uniqueness — the three measures
//! that close the perkolation gate `G_perc`. Lattice ids are
//! permutation-invariant by virtue of sorting before hashing.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::{fixed_ge, CognitionError, Fixed};
use super::run_descriptor::CognitionRunDescriptor;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AdmissibleRegion {
    pub region_id: Hash256,
    pub support_hash: Hash256,
    pub mass: Fixed,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Resonite {
    pub resonite_id: Hash256,
    pub carrier_constraint: Hash256,
    pub resonance_weight: Fixed,
    pub supports: Vec<Hash256>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum InfogenePolicy {
    Stable,
    Mutable,
    Frozen,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Infogene {
    pub infogene_id: Hash256,
    pub feature_hash: Hash256,
    pub constraint_hashes: Vec<Hash256>,
    pub mutation_policy: InfogenePolicy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstraintLatticeCognition {
    pub lattice_id: Hash256,
    pub admissible_regions: Vec<AdmissibleRegion>,
    pub resonites: Vec<Resonite>,
    pub infogenes: Vec<Infogene>,
    pub weighted_constraint_mass: Fixed,
    pub entropy_reduction: Fixed,
    pub feasible_set_uniqueness: Fixed,
    pub percolation_passed: bool,
}

impl ConstraintLatticeCognition {
    /// Sort every list before hashing so permuted callers produce the
    /// same `lattice_id` (spec property test §21.3.1).
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        self.admissible_regions.sort();
        self.resonites.sort();
        self.infogenes.sort();
        let mut probe = self.clone();
        probe.lattice_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.lattice_id = Hash256(id);
        Ok(self)
    }
}

/// Per §9.2 — `G_perc = 1 ⇔ M_c ≥ θ_M ∧ ΔH ≥ θ_H ∧ U_F ≥ θ_U`.
pub fn evaluate_perkolation(
    lattice: &ConstraintLatticeCognition,
    rd: &CognitionRunDescriptor,
) -> bool {
    fixed_ge(
        &lattice.weighted_constraint_mass,
        &rd.thresholds.min_constraint_mass,
    ) && fixed_ge(
        &lattice.entropy_reduction,
        &rd.thresholds.min_entropy_reduction,
    ) && fixed_ge(
        &lattice.feasible_set_uniqueness,
        &rd.thresholds.min_feasible_uniqueness,
    )
}

/// Build a minimal lattice from a small support payload, computing the
/// three perkolation measures from the input. This is the reference
/// constructor used by the pipeline; production callers may supply a
/// richer constructor as long as the resulting lattice respects the
/// `with_id` invariant.
pub fn build_lattice_minimal(
    rd: &CognitionRunDescriptor,
    constraint_count: u32,
    support_strength: &Fixed,
) -> Result<ConstraintLatticeCognition, CognitionError> {
    let mass = Fixed::Rational {
        num: constraint_count as i128,
        den: 1,
    };
    let weighted_constraint_mass = mul(&mass, support_strength);
    let entropy_reduction = support_strength.clone();
    let feasible_set_uniqueness = if constraint_count == 0 {
        Fixed::quantize(0.0, 9).unwrap()
    } else {
        Fixed::Rational {
            num: 1,
            den: constraint_count as i128,
        }
    };

    let lattice = ConstraintLatticeCognition {
        lattice_id: Hash256::zero(),
        admissible_regions: Vec::new(),
        resonites: Vec::new(),
        infogenes: Vec::new(),
        weighted_constraint_mass,
        entropy_reduction,
        feasible_set_uniqueness,
        percolation_passed: false,
    };
    let mut lattice = lattice.with_id()?;
    lattice.percolation_passed = evaluate_perkolation(&lattice, rd);
    lattice.with_id()
}

fn mul(a: &Fixed, b: &Fixed) -> Fixed {
    let (an, ad) = pair(a);
    let (bn, bd) = pair(b);
    super::primitives::normalize_rational(an.saturating_mul(bn), ad.saturating_mul(bd))
}

fn pair(x: &Fixed) -> (i128, i128) {
    match x {
        Fixed::FixedI64 { raw, scale } => (*raw as i128, 10i128.pow(*scale)),
        Fixed::Rational { num, den } => (*num, *den),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::run_descriptor::{CognitionPolicies, CognitionThresholds};
    use std::collections::BTreeMap;

    fn rd() -> CognitionRunDescriptor {
        CognitionRunDescriptor {
            run_id: "cognition.test".into(),
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
    fn perkolation_requires_all_terms() {
        let mut rd = rd();
        rd.thresholds.min_constraint_mass = Fixed::quantize(10.0, 9).unwrap();
        let lattice = build_lattice_minimal(&rd, 4, &Fixed::quantize(0.5, 9).unwrap()).unwrap();
        assert!(!lattice.percolation_passed);
    }

    #[test]
    fn permuted_constraints_yield_identical_lattice_id() {
        let rd = rd();
        let mut a = build_lattice_minimal(&rd, 4, &Fixed::quantize(0.5, 9).unwrap()).unwrap();
        // Add a couple of resonites in two orders.
        let r1 = Resonite {
            resonite_id: {
                let mut x = [0u8; 32];
                x[0] = 1;
                Hash256(x)
            },
            carrier_constraint: Hash256::zero(),
            resonance_weight: Fixed::quantize(0.1, 9).unwrap(),
            supports: vec![],
        };
        let r2 = Resonite {
            resonite_id: {
                let mut x = [0u8; 32];
                x[0] = 2;
                Hash256(x)
            },
            carrier_constraint: Hash256::zero(),
            resonance_weight: Fixed::quantize(0.2, 9).unwrap(),
            supports: vec![],
        };
        let mut b = a.clone();
        a.resonites = vec![r1.clone(), r2.clone()];
        b.resonites = vec![r2, r1];
        let a = a.with_id().unwrap();
        let b = b.with_id().unwrap();
        assert_eq!(a.lattice_id, b.lattice_id);
    }
}

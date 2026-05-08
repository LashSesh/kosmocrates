//! Layer C2 — Operator Algebra and Simulation Kernel (§7).
//!
//! Four base operator families: `DoubleKick`, `ThresholdSweep`,
//! `PathInvariance`, `WormholeFunnel`. Each operator declares its
//! domain/codomain and a list of declarative invariants. The kernel
//! verifies type compatibility deterministically.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::{CognitionError, Fixed, StableId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OperatorType {
    State5D,
    SpiralPath,
    ConstraintLattice,
    HypercubePuzzle,
    PhasePanorama,
    SelfModel,
    CandidateBundle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OperatorFamily {
    DoubleKick,
    ThresholdSweep,
    PathInvariance,
    WormholeFunnel,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct InvariantSpec {
    pub invariant_id: StableId,
    pub kind: String,
    pub description: String,
}

/// Spec-§7.1 operator declaration. Implementations live higher up
/// (apply runs in the pipeline); this type is the audit-pathable
/// declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorDeclaration {
    pub operator_id: StableId,
    pub family: OperatorFamily,
    pub domain: OperatorType,
    pub codomain: OperatorType,
    pub invariants: Vec<InvariantSpec>,
    pub declared_hash: Hash256,
}

impl OperatorDeclaration {
    pub fn new(
        operator_id: impl Into<String>,
        family: OperatorFamily,
        domain: OperatorType,
        codomain: OperatorType,
    ) -> Result<Self, CognitionError> {
        let operator_id = operator_id.into();
        let invariants = Vec::new();
        let mut decl = OperatorDeclaration {
            operator_id,
            family,
            domain,
            codomain,
            invariants,
            declared_hash: Hash256::zero(),
        };
        decl.declared_hash = Hash256(
            content_address(&(
                &decl.operator_id,
                &decl.family,
                &decl.domain,
                &decl.codomain,
            ))
            .map_err(|e| CognitionError::Canonicalization {
                reason: e.to_string(),
            })?,
        );
        Ok(decl)
    }

    /// Type-check a sequential composition `self ∘ next`.
    pub fn compose(&self, next: &OperatorDeclaration) -> Result<(), CognitionError> {
        if self.codomain != next.domain {
            return Err(CognitionError::OperatorTypeMismatch {
                reason: format!(
                    "compose: {:?}.codomain={:?} != {:?}.domain={:?}",
                    self.operator_id, self.codomain, next.operator_id, next.domain
                ),
            });
        }
        Ok(())
    }
}

/// `IntegrationMode` per §7.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IntegrationMode {
    Compose,
    Overlay,
    Inject,
    Absorb,
}

/// `IntegratorKind` per §7.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IntegratorKind {
    FixedHeun,
    FixedEulerForTests,
    NoFloatAnalyticProxy,
}

/// Deterministic simulation spec — every input is content-addressed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionSimulationSpec {
    pub initial_state_hash: Hash256,
    pub coupling_matrix_hash: Hash256,
    pub vector_field_hash: Hash256,
    pub integrator: IntegratorKind,
    pub dt: Fixed,
    pub max_steps: u64,
    pub stability_epsilon: Fixed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_declaration_is_deterministic() {
        let a = OperatorDeclaration::new(
            "double_kick.test",
            OperatorFamily::DoubleKick,
            OperatorType::State5D,
            OperatorType::State5D,
        )
        .unwrap();
        let b = OperatorDeclaration::new(
            "double_kick.test",
            OperatorFamily::DoubleKick,
            OperatorType::State5D,
            OperatorType::State5D,
        )
        .unwrap();
        assert_eq!(a.declared_hash, b.declared_hash);
    }

    #[test]
    fn operator_algebra_rejects_type_mismatch() {
        let a = OperatorDeclaration::new(
            "a",
            OperatorFamily::DoubleKick,
            OperatorType::State5D,
            OperatorType::State5D,
        )
        .unwrap();
        let b = OperatorDeclaration::new(
            "b",
            OperatorFamily::ThresholdSweep,
            OperatorType::ConstraintLattice,
            OperatorType::ConstraintLattice,
        )
        .unwrap();
        assert!(a.compose(&b).is_err());
    }
}

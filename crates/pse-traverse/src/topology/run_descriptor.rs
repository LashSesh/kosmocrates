//! TptMtlRunDescriptor — the single replay anchor for the topology layer.
//!
//! Two runs with byte-identical descriptors and inputs MUST produce
//! byte-identical digests for Window, Mesh, MicroFibers, GateReport and Bundle
//! (invariant I-10).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::primitives::{tpt_content_address, Fixed, Hash256, TopologyError};

/// Policy governing numeric representation in audit/gate paths.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum NumericPolicy {
    Fixed,
    Rational,
    BoundedApprox { tolerance: Fixed },
}

/// Tie-break policy for deterministic ordering when digest ordering is ambiguous.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TieBreakPolicy {
    DigestLexicographic,
    StableIndexThenDigest,
}

/// Axis separation policy (invariant I-03).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AxisPolicy {
    /// Semantic 5D axis labels (R, F, T, S, E or profile-defined equivalents).
    pub semantic_axes: [String; 5],
    /// Runtime operative axis labels (psi, rho, omega, chi, tau).
    pub runtime_axes: [String; 5],
    /// Carrier metadata axis labels (n0, t2, t1, alpha, beta, mandorla, lambda, …).
    pub carrier_axes: Vec<String>,
    /// Declared translation policy between axis layers.
    pub translation_policy: String,
}

impl AxisPolicy {
    pub fn default_profile() -> Self {
        AxisPolicy {
            semantic_axes: [
                "R".into(),
                "F".into(),
                "T".into(),
                "S".into(),
                "E".into(),
            ],
            runtime_axes: [
                "psi".into(),
                "rho".into(),
                "omega".into(),
                "chi".into(),
                "tau".into(),
            ],
            carrier_axes: vec![
                "n0".into(),
                "t2".into(),
                "t1".into(),
                "alpha".into(),
                "beta".into(),
                "mandorla".into(),
                "lambda".into(),
            ],
            translation_policy: "identity".into(),
        }
    }
}

/// Mesh triangulation policy parameters.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MeshPolicy {
    /// Cost function coefficients α1..α7 (declared; unimplemented terms
    /// reduce conformance class).
    pub alpha_symmetry: Fixed,
    pub alpha_resolution: Fixed,
    pub alpha_topo_stab: Fixed,
    pub alpha_seam: Fixed,
    pub alpha_carrier: Fixed,
    pub alpha_entropy_penalty: Fixed,
    pub alpha_complexity: Fixed,
    /// Maximum allowed Betti shift before TopologyGuard fires.
    pub max_betti_shift: Vec<i64>,
    /// Wasserstein persistence diagram distance threshold.
    pub max_pd_wasserstein: Fixed,
}

impl MeshPolicy {
    pub fn permissive() -> Self {
        let zero = Fixed::quantize(0.0, 9).unwrap();
        let one = Fixed::quantize(1.0, 9).unwrap();
        let big = Fixed::quantize(1000.0, 9).unwrap();
        MeshPolicy {
            alpha_symmetry: one.clone(),
            alpha_resolution: one.clone(),
            alpha_topo_stab: one.clone(),
            alpha_seam: one.clone(),
            alpha_carrier: one.clone(),
            alpha_entropy_penalty: zero.clone(),
            alpha_complexity: zero,
            max_betti_shift: vec![2, 2, 2, 2, 2],
            max_pd_wasserstein: big,
        }
    }
}

/// Micro-lift policy parameters.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MicroLiftPolicy {
    /// Dualization rule id (e.g. "MTL-D1").
    pub dualization_rule: String,
    /// Neighbourhood metric for subwindows.
    pub neighborhood_metric: String,
    /// k-nearest or radius — encoded as string to stay platform-agnostic.
    pub k_or_radius: String,
    /// Epsilon for MTL-D1 denominator guard.
    pub epsilon: Fixed,
}

impl MicroLiftPolicy {
    pub fn mtl_d1_default() -> Self {
        MicroLiftPolicy {
            dualization_rule: "MTL-D1".into(),
            neighborhood_metric: "euclidean".into(),
            k_or_radius: "k=5".into(),
            epsilon: Fixed::quantize(1e-9, 9).unwrap(),
        }
    }
}

/// Resource budgets for bounded recursion (invariant I-09).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TptMtlBudgets {
    pub max_depth: u32,
    pub max_steps: u64,
    pub max_vertices: u64,
    pub max_simplices: u64,
    pub max_time_ms: u64,
    pub max_memory_kb: u64,
}

impl TptMtlBudgets {
    pub fn permissive() -> Self {
        TptMtlBudgets {
            max_depth: 8,
            max_steps: 4096,
            max_vertices: 1024,
            max_simplices: 4096,
            max_time_ms: 30_000,
            max_memory_kb: 512_000,
        }
    }
}

/// Gate threshold parameters.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TptMtlThresholds {
    pub min_adapter_totality: Fixed,
    pub min_axis_bridge_validity: Fixed,
    pub min_mesh_determinism: Fixed,
    pub min_micro_lift_coverage: Fixed,
    pub min_seam_consistency: Fixed,
    pub min_carrier_continuity: Fixed,
    pub max_false_crystal_rate: Fixed,
    pub min_trace_completeness: Fixed,
    pub min_replay_identity: Fixed,
}

impl TptMtlThresholds {
    pub fn permissive() -> Self {
        let zero = Fixed::quantize(0.0, 9).unwrap();
        let one = Fixed::quantize(1.0, 9).unwrap();
        TptMtlThresholds {
            min_adapter_totality: zero.clone(),
            min_axis_bridge_validity: zero.clone(),
            min_mesh_determinism: zero.clone(),
            min_micro_lift_coverage: zero.clone(),
            min_seam_consistency: zero.clone(),
            min_carrier_continuity: zero.clone(),
            max_false_crystal_rate: one.clone(),
            min_trace_completeness: zero.clone(),
            min_replay_identity: zero,
        }
    }
}

/// The single replay anchor for the topology layer (TPT-MTL §4.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TptMtlRunDescriptor {
    /// Spec version — "TPT-MTL-0.4".
    pub rdver: String,
    /// Deterministic seed — no entropy claim.
    pub seed: String,
    /// Canonicalization scheme — e.g. "jcs-rfc8785".
    pub canonicalization: String,
    pub numeric_policy: NumericPolicy,
    pub tie_break: TieBreakPolicy,
    pub domain_profile: String,
    /// Declared source boundary digests.
    pub source_boundaries: Vec<Hash256>,
    /// Operator version registry (BTreeMap for determinism).
    pub operator_versions: BTreeMap<String, String>,
    pub budgets: TptMtlBudgets,
    pub axis_policy: AxisPolicy,
    pub mesh_policy: MeshPolicy,
    pub micro_lift_policy: MicroLiftPolicy,
    pub gate_thresholds: TptMtlThresholds,
}

impl TptMtlRunDescriptor {
    /// Validate structural invariants.
    pub fn validate(&self) -> Result<(), TopologyError> {
        if self.rdver != "TPT-MTL-0.4" {
            return Err(TopologyError::InvalidDescriptor {
                reason: format!("expected rdver TPT-MTL-0.4, got {}", self.rdver),
            });
        }
        if self.canonicalization.is_empty() {
            return Err(TopologyError::InvalidDescriptor {
                reason: "canonicalization is empty".into(),
            });
        }
        if self.domain_profile.is_empty() {
            return Err(TopologyError::InvalidDescriptor {
                reason: "domain_profile is empty".into(),
            });
        }
        if self.budgets.max_steps == 0 {
            return Err(TopologyError::InvalidDescriptor {
                reason: "max_steps must be >= 1".into(),
            });
        }
        Ok(())
    }

    /// Canonical content address of this descriptor — used as replay anchor.
    pub fn content_hash(&self) -> Result<Hash256, TopologyError> {
        tpt_content_address(self)
    }

    /// Construct a permissive default descriptor for tests.
    pub fn default_permissive() -> Self {
        TptMtlRunDescriptor {
            rdver: "TPT-MTL-0.4".into(),
            seed: "deterministic-0".into(),
            canonicalization: "jcs-rfc8785".into(),
            numeric_policy: NumericPolicy::Fixed,
            tie_break: TieBreakPolicy::DigestLexicographic,
            domain_profile: "default".into(),
            source_boundaries: vec![],
            operator_versions: BTreeMap::new(),
            budgets: TptMtlBudgets::permissive(),
            axis_policy: AxisPolicy::default_profile(),
            mesh_policy: MeshPolicy::permissive(),
            micro_lift_policy: MicroLiftPolicy::mtl_d1_default(),
            gate_thresholds: TptMtlThresholds::permissive(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_validates_correctly() {
        let rd = TptMtlRunDescriptor::default_permissive();
        assert!(rd.validate().is_ok());
    }

    #[test]
    fn descriptor_rejects_wrong_version() {
        let mut rd = TptMtlRunDescriptor::default_permissive();
        rd.rdver = "TPT-MTL-0.3".into();
        assert!(rd.validate().is_err());
    }

    #[test]
    fn descriptor_content_hash_is_deterministic() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let h1 = rd.content_hash().unwrap();
        let h2 = rd.content_hash().unwrap();
        assert_eq!(h1, h2);
    }
}

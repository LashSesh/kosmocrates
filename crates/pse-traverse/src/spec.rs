//! `ProblemSpec` and supporting types — the normalised input contract.
//!
//! A prompt may *produce* a `ProblemSpec` but never *replaces* it. The
//! traversal kernel reads only structured specs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level problem specification. Loaded from JSON, content-addressable
/// via [`crate::content_address`], and the only legitimate input to
/// [`crate::DefaultFieldCubeBuilder`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProblemSpec {
    pub id: String,
    pub title: String,
    pub objective: String,
    pub domain: String,
    pub inputs: Vec<InputRef>,
    pub constraints: Vec<ConstraintSpec>,
    pub dimensions: Vec<DimensionSpec>,
    pub desired_outputs: Vec<OutputSpec>,
    pub risk_policy: RiskPolicy,
    pub replay: ReplayPolicy,
    pub metadata: BTreeMap<String, String>,
}

/// External input pointer — file, URL, in-memory blob ID, etc.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InputRef {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Constraint on the problem space. `predicate` is a free-form identifier
/// resolved by the constraint registry (Phase 4 will materialise it as a
/// proper trait); for the v0.1 MVP the predicate is opaque metadata.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ConstraintSpec {
    pub id: String,
    pub kind: ConstraintKind,
    pub predicate: String,
    pub weight: f64,
    /// Optional list of dimension IDs this constraint couples.
    #[serde(default)]
    pub dimensions: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ConstraintKind {
    /// Hard: violation rejects.
    Hard,
    /// Soft: violation degrades a score but does not reject.
    Soft,
    /// Boundary: marks the edge of the admissible region.
    Boundary,
}

/// One degree of freedom — a dimension along which the problem can vary.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DimensionSpec {
    pub id: String,
    pub label: String,
    pub kind: DimensionKind,
    pub values: ValueDomain,
    pub required: bool,
    pub source: DimensionSource,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DimensionKind {
    Boolean,
    Enum,
    Ordinal,
    Continuous,
    Hierarchical,
    Derived,
}

/// Provenance of a dimension — where it came from in the input.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DimensionSource {
    User,
    Inferred,
    External,
    Derived,
}

/// Value domain for a dimension.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ValueDomain {
    Boolean,
    Enum(Vec<String>),
    Range {
        min: f64,
        max: f64,
        step: Option<f64>,
    },
    Tree(Vec<TreeValue>),
    External {
        resolver: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TreeValue {
    pub label: String,
    #[serde(default)]
    pub children: Vec<TreeValue>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OutputSpec {
    pub id: String,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RiskPolicy {
    pub fail_closed: bool,
    pub allow_oracle: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReplayPolicy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    pub canonical: bool,
}

impl Default for RiskPolicy {
    fn default() -> Self {
        // Conservative defaults: fail-closed, no oracles.
        Self {
            fail_closed: true,
            allow_oracle: false,
        }
    }
}

impl Default for ReplayPolicy {
    fn default() -> Self {
        Self {
            seed: None,
            canonical: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::content_address;

    fn minimal_spec() -> ProblemSpec {
        ProblemSpec {
            id: "test.minimal.v1".into(),
            title: "Minimal".into(),
            objective: "Test".into(),
            domain: "test".into(),
            inputs: vec![],
            constraints: vec![ConstraintSpec {
                id: "c.demo".into(),
                kind: ConstraintKind::Hard,
                predicate: "always_true".into(),
                weight: 1.0,
                dimensions: vec!["d.layout".into()],
            }],
            dimensions: vec![DimensionSpec {
                id: "d.layout".into(),
                label: "Layout".into(),
                kind: DimensionKind::Enum,
                values: ValueDomain::Enum(vec!["a".into(), "b".into()]),
                required: true,
                source: DimensionSource::User,
            }],
            desired_outputs: vec![OutputSpec {
                id: "o.plan".into(),
                kind: "collapse_plan".into(),
            }],
            risk_policy: RiskPolicy::default(),
            replay: ReplayPolicy {
                seed: Some(42),
                canonical: true,
            },
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn problem_spec_roundtrips() {
        let s = minimal_spec();
        let bytes = serde_json::to_vec(&s).unwrap();
        let back: ProblemSpec = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn problem_spec_canonical_address_stable() {
        let s = minimal_spec();
        let h1 = content_address(&s).unwrap();
        let h2 = content_address(&s).unwrap();
        assert_eq!(h1, h2);
    }
}

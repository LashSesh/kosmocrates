//! Path-excision detection — the formal-vs-operative-option distinction.
//!
//! A *formal* option is a value in a dimension's value domain.
//! An *operative* option is a value reachable through at least one
//! admissible path. The two are not the same; the difference is what
//! turns "lots of choices" into "actually decidable."

use serde::{Deserialize, Serialize};

use crate::field_cube::{EvidenceRef, FieldCube};
use crate::spec::ValueDomain;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PathExcision {
    pub dimension_id: String,
    pub formal_value: String,
    pub reason: String,
    pub missing_requirements: Vec<String>,
    pub blocked_by_constraints: Vec<String>,
    pub operational_impact: OperationalImpact,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum OperationalImpact {
    Low,
    Medium,
    High,
    Critical,
}

/// Detect path excisions over a FieldCube.
///
/// MVP semantics: for every required dimension, every formal value in
/// the domain is admissible iff at least one *path* in the cube targets
/// the dimension and is marked admissible. If no admissible path
/// targets the dimension, every formal value of that dimension is
/// emitted as a `PathExcision` with `reason="no_admissible_path"`.
///
/// Output is deterministic: sorted by `(dimension_id, formal_value)`.
pub fn detect_path_excision(cube: &FieldCube) -> Vec<PathExcision> {
    let mut out = Vec::new();
    for (dim_id, dim) in &cube.dimensions {
        if !dim.required {
            continue;
        }
        let admissible_paths_to_dim: Vec<&crate::field_cube::PathSpec> = cube
            .paths
            .iter()
            .filter(|p| &p.to == dim_id && p.admissible)
            .collect();
        if !admissible_paths_to_dim.is_empty() {
            continue;
        }
        let blocking_paths: Vec<String> = cube
            .paths
            .iter()
            .filter(|p| &p.to == dim_id)
            .flat_map(|p| p.gating_constraints.iter().cloned())
            .collect();
        let mut blocking = blocking_paths;
        blocking.sort();
        blocking.dedup();
        for value in enumerate_values(&dim.values) {
            out.push(PathExcision {
                dimension_id: dim_id.clone(),
                formal_value: value,
                reason: "no_admissible_path".into(),
                missing_requirements: vec!["admissible_path_to_target".into()],
                blocked_by_constraints: blocking.clone(),
                operational_impact: OperationalImpact::High,
                evidence: Vec::new(),
            });
        }
    }
    out.sort_by(|a, b| {
        a.dimension_id
            .cmp(&b.dimension_id)
            .then(a.formal_value.cmp(&b.formal_value))
    });
    out
}

fn enumerate_values(d: &ValueDomain) -> Vec<String> {
    match d {
        ValueDomain::Boolean => vec!["false".into(), "true".into()],
        ValueDomain::Enum(v) => {
            let mut s = v.clone();
            s.sort();
            s
        }
        ValueDomain::Range { min, max, step } => {
            // Surface the endpoints as representative formal values; finer
            // sampling is a tuning concern not relevant to excision detection.
            let step_label = step.map(|s| format!(",step={}", s)).unwrap_or_default();
            vec![format!("min={}", min), format!("max={}{}", max, step_label)]
        }
        ValueDomain::Tree(tree) => {
            let mut labels: Vec<String> = tree.iter().map(|t| t.label.clone()).collect();
            labels.sort();
            labels
        }
        ValueDomain::External { resolver } => vec![format!("external:{}", resolver)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_cube::{DefaultFieldCubeBuilder, FieldCubeBuilder};
    use crate::spec::{
        ConstraintKind, ConstraintSpec, DimensionKind, DimensionSource, DimensionSpec, OutputSpec,
        ProblemSpec, ReplayPolicy, RiskPolicy,
    };
    use std::collections::BTreeMap;

    fn spec_with_no_constraint_for_dimension() -> ProblemSpec {
        ProblemSpec {
            id: "p.excision".into(),
            title: "T".into(),
            objective: "o".into(),
            domain: "d".into(),
            inputs: vec![],
            constraints: vec![],
            dimensions: vec![DimensionSpec {
                id: "d.unreachable".into(),
                label: "U".into(),
                kind: DimensionKind::Enum,
                values: ValueDomain::Enum(vec!["x".into(), "y".into()]),
                required: true,
                source: DimensionSource::User,
            }],
            desired_outputs: vec![OutputSpec {
                id: "o.x".into(),
                kind: "y".into(),
            }],
            risk_policy: RiskPolicy::default(),
            replay: ReplayPolicy::default(),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn detects_path_excision_when_no_admissible_path() {
        let spec = spec_with_no_constraint_for_dimension();
        let cube = DefaultFieldCubeBuilder.build(&spec).unwrap();
        let excisions = detect_path_excision(&cube);
        assert_eq!(excisions.len(), 2);
        assert_eq!(excisions[0].dimension_id, "d.unreachable");
        assert_eq!(excisions[0].reason, "no_admissible_path");
    }

    #[test]
    fn detect_path_excision_is_deterministic() {
        let cube = DefaultFieldCubeBuilder
            .build(&spec_with_no_constraint_for_dimension())
            .unwrap();
        let a = detect_path_excision(&cube);
        let b = detect_path_excision(&cube);
        assert_eq!(a, b);
    }

    #[test]
    fn no_excision_when_constraint_couples_required_dimension() {
        let mut spec = spec_with_no_constraint_for_dimension();
        spec.constraints.push(ConstraintSpec {
            id: "c.bind".into(),
            kind: ConstraintKind::Hard,
            predicate: "true".into(),
            weight: 1.0,
            dimensions: vec!["d.unreachable".into()],
        });
        let cube = DefaultFieldCubeBuilder.build(&spec).unwrap();
        let excisions = detect_path_excision(&cube);
        assert!(excisions.is_empty(), "got {:?}", excisions);
    }
}

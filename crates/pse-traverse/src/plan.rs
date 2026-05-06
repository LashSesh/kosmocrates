//! `CollapsePlan` — deterministic ordering of collapse steps over the
//! degree-of-freedom graph.
//!
//! Ordering policy (MVP):
//!  1. Hard constraint steps before soft.
//!  2. Steps targeting high-coupling nodes before low-coupling.
//!  3. Boundary / path-excision checks before solver execution.
//!  4. Verification before commit.
//!  5. Lexicographic ID tie-break.

use serde::{Deserialize, Serialize};

use crate::dof::{DoFGraph, NodeId};
use crate::excision::PathExcision;
use crate::field_cube::{EvidenceRef, FieldCube};
use crate::spec::ConstraintKind;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CollapsePlan {
    pub id: String,
    pub field_cube_id: String,
    pub steps: Vec<CollapseStep>,
    pub ordering_policy: OrderingPolicy,
    pub expected_reduction: f64,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CollapseStep {
    pub id: String,
    pub kind: CollapseStepKind,
    pub target_nodes: Vec<NodeId>,
    pub required_constraints: Vec<String>,
    pub expected_effect: CollapseEffect,
    pub failure_policy: FailurePolicy,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CollapseStepKind {
    ResolveDimension,
    ApplyConstraint,
    SplitComponent,
    MergeComponent,
    DetectPathExcision,
    MigrateCarrier,
    VerifyCandidate,
    CommitCandidate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CollapseEffect {
    /// 0.0–1.0 estimated reduction in remaining freedom this step achieves.
    pub estimated_reduction: f64,
    /// Free-form structural label.
    pub kind: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FailurePolicy {
    /// Fail closed — reject candidate, do not commit.
    FailClosed,
    /// Convert failure into a refinement request.
    Refine,
    /// Convert failure into a path-excision report.
    Excise,
    /// Convert failure into a boundary report.
    Boundary,
    /// Abort the whole run.
    Abort,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OrderingPolicy {
    /// MVP policy described in `crates/pse-traverse/src/plan.rs`.
    DeterministicLexCoupling,
}

pub trait CollapsePlanner {
    fn plan(
        &self,
        cube: &FieldCube,
        graph: &DoFGraph,
        excisions: &[PathExcision],
    ) -> CollapsePlan;
}

/// Reference planner — implements the MVP ordering policy.
pub struct DefaultCollapsePlanner;

impl CollapsePlanner for DefaultCollapsePlanner {
    fn plan(
        &self,
        cube: &FieldCube,
        graph: &DoFGraph,
        excisions: &[PathExcision],
    ) -> CollapsePlan {
        let mut steps: Vec<CollapseStep> = Vec::new();

        // 1. Path-excision detection step always comes first (it's a check, not a mutation).
        steps.push(CollapseStep {
            id: "s.detect_path_excision".into(),
            kind: CollapseStepKind::DetectPathExcision,
            target_nodes: excisions.iter()
                .map(|e| NodeId::new(&e.dimension_id))
                .collect(),
            required_constraints: Vec::new(),
            expected_effect: CollapseEffect {
                estimated_reduction: 0.0,
                kind: "diagnose".into(),
            },
            failure_policy: FailurePolicy::Excise,
        });

        // 2. Hard-constraint application steps, sorted by coupled-node degree desc, ID asc.
        let mut hard: Vec<(String, u64)> = cube.constraints.values()
            .filter(|c| c.kind == ConstraintKind::Hard)
            .map(|c| {
                let max_deg = c.dimensions.iter()
                    .map(|d| graph.degree(&NodeId::new(d)))
                    .max()
                    .unwrap_or(0);
                (c.id.clone(), max_deg)
            })
            .collect();
        hard.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        for (cid, _deg) in &hard {
            steps.push(CollapseStep {
                id: format!("s.apply.{}", cid),
                kind: CollapseStepKind::ApplyConstraint,
                target_nodes: vec![NodeId::new(cid)],
                required_constraints: vec![cid.clone()],
                expected_effect: CollapseEffect {
                    estimated_reduction: 0.05,
                    kind: "apply_hard".into(),
                },
                failure_policy: FailurePolicy::FailClosed,
            });
        }

        // 3. Soft-constraint steps (lex sort).
        let mut soft: Vec<&str> = cube.constraints.values()
            .filter(|c| c.kind == ConstraintKind::Soft)
            .map(|c| c.id.as_str())
            .collect();
        soft.sort();
        for cid in soft {
            steps.push(CollapseStep {
                id: format!("s.apply.{}", cid),
                kind: CollapseStepKind::ApplyConstraint,
                target_nodes: vec![NodeId::new(cid)],
                required_constraints: vec![cid.into()],
                expected_effect: CollapseEffect {
                    estimated_reduction: 0.02,
                    kind: "apply_soft".into(),
                },
                failure_policy: FailurePolicy::Refine,
            });
        }

        // 4. Required-dimension resolution steps (lex sort).
        let mut dims: Vec<&str> = cube.dimensions.values()
            .filter(|d| d.required)
            .map(|d| d.id.as_str())
            .collect();
        dims.sort();
        for did in dims {
            steps.push(CollapseStep {
                id: format!("s.resolve.{}", did),
                kind: CollapseStepKind::ResolveDimension,
                target_nodes: vec![NodeId::new(did)],
                required_constraints: cube.constraints.values()
                    .filter(|c| c.dimensions.iter().any(|d| d == did))
                    .map(|c| c.id.clone())
                    .collect(),
                expected_effect: CollapseEffect {
                    estimated_reduction: 0.10,
                    kind: "resolve_dimension".into(),
                },
                failure_policy: FailurePolicy::FailClosed,
            });
        }

        // 5. Verification step.
        steps.push(CollapseStep {
            id: "s.verify_candidate".into(),
            kind: CollapseStepKind::VerifyCandidate,
            target_nodes: Vec::new(),
            required_constraints: Vec::new(),
            expected_effect: CollapseEffect {
                estimated_reduction: 0.0,
                kind: "verify".into(),
            },
            failure_policy: FailurePolicy::FailClosed,
        });

        // 6. Commit step.
        steps.push(CollapseStep {
            id: "s.commit_candidate".into(),
            kind: CollapseStepKind::CommitCandidate,
            target_nodes: Vec::new(),
            required_constraints: Vec::new(),
            expected_effect: CollapseEffect {
                estimated_reduction: 0.0,
                kind: "commit".into(),
            },
            failure_policy: FailurePolicy::FailClosed,
        });

        let expected_reduction = steps.iter()
            .map(|s| s.expected_effect.estimated_reduction)
            .sum::<f64>()
            .min(1.0);

        CollapsePlan {
            id: format!("plan.{}", cube.id),
            field_cube_id: cube.id.clone(),
            steps,
            ordering_policy: OrderingPolicy::DeterministicLexCoupling,
            expected_reduction,
            evidence: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::content_address;
    use crate::dof::DoFGraph;
    use crate::excision::detect_path_excision;
    use crate::field_cube::{DefaultFieldCubeBuilder, FieldCubeBuilder};
    use crate::spec::{
        ConstraintSpec, DimensionKind, DimensionSource, DimensionSpec,
        OutputSpec, ProblemSpec, ReplayPolicy, RiskPolicy, ValueDomain,
    };
    use std::collections::BTreeMap;

    fn spec() -> ProblemSpec {
        ProblemSpec {
            id: "p.plan".into(),
            title: "T".into(),
            objective: "o".into(),
            domain: "d".into(),
            inputs: vec![],
            constraints: vec![
                ConstraintSpec {
                    id: "c.b".into(),
                    kind: ConstraintKind::Hard,
                    predicate: "p".into(),
                    weight: 1.0,
                    dimensions: vec!["d.x".into()],
                },
                ConstraintSpec {
                    id: "c.a".into(),
                    kind: ConstraintKind::Soft,
                    predicate: "q".into(),
                    weight: 0.5,
                    dimensions: vec!["d.x".into()],
                },
            ],
            dimensions: vec![DimensionSpec {
                id: "d.x".into(),
                label: "X".into(),
                kind: DimensionKind::Boolean,
                values: ValueDomain::Boolean,
                required: true,
                source: DimensionSource::User,
            }],
            desired_outputs: vec![OutputSpec { id: "o.x".into(), kind: "y".into() }],
            risk_policy: RiskPolicy::default(),
            replay: ReplayPolicy::default(),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn collapse_plan_stable_ordering() {
        let cube = DefaultFieldCubeBuilder.build(&spec()).unwrap();
        let g = DoFGraph::from_field_cube(&cube);
        let exc = detect_path_excision(&cube);
        let p1 = DefaultCollapsePlanner.plan(&cube, &g, &exc);
        let p2 = DefaultCollapsePlanner.plan(&cube, &g, &exc);
        assert_eq!(p1, p2);
        assert_eq!(content_address(&p1).unwrap(), content_address(&p2).unwrap());
    }

    #[test]
    fn plan_orders_hard_before_soft_and_excision_first() {
        let cube = DefaultFieldCubeBuilder.build(&spec()).unwrap();
        let g = DoFGraph::from_field_cube(&cube);
        let exc = detect_path_excision(&cube);
        let p = DefaultCollapsePlanner.plan(&cube, &g, &exc);
        let kinds: Vec<CollapseStepKind> = p.steps.iter().map(|s| s.kind).collect();
        assert_eq!(kinds[0], CollapseStepKind::DetectPathExcision);
        // hard constraint c.b should come before soft constraint c.a
        let i_hard = p.steps.iter().position(|s| s.id == "s.apply.c.b").unwrap();
        let i_soft = p.steps.iter().position(|s| s.id == "s.apply.c.a").unwrap();
        assert!(i_hard < i_soft, "hard {} not before soft {}", i_hard, i_soft);
        // verify before commit
        let i_v = p.steps.iter().position(|s| s.kind == CollapseStepKind::VerifyCandidate).unwrap();
        let i_c = p.steps.iter().position(|s| s.kind == CollapseStepKind::CommitCandidate).unwrap();
        assert!(i_v < i_c);
    }
}

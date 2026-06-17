//! Stufe 2, brick 48 — the bridge to `pse-traverse`.
//!
//! The pre-kosmocrates topological substrate (`pse-traverse`) is, at its core,
//! already a deterministic conformance machine for *abstract* problem spaces:
//! a [`ProblemSpec`] becomes a `FieldCube`, a `DoFGraph`, and a `CollapsePlan`
//! that drives the degrees of freedom to zero, fail-closed, with `PathExcision`
//! for formally-wished-but-operationally-unreachable options.
//!
//! This module *specialises* that core to the software-architecture problem
//! space. The mapping is **clean, not an arbitrary embedding**: every facet of
//! the architecture is a Boolean degree of freedom (realized or not); every
//! `depends_on` relation (the same one the blueprint draws) is a hard constraint
//! coupling the dependent to its foundation. So the `ProblemSpec` is a 1:1 image
//! of the blueprint graph, and the `CollapsePlan` is the deterministic,
//! foundations-first order in which to realize it. We consume `pse-traverse` as
//! a library — no fork, no copy.

use kosmo_core::{depends_on, WishFacet};
use pse_traverse::{
    detect_path_excision, CollapsePlanner, CollapseStepKind, ConstraintKind, ConstraintSpec,
    DefaultCollapsePlanner, DefaultFieldCubeBuilder, DimensionKind, DimensionSource, DimensionSpec,
    DoFGraph, FieldCubeBuilder, OperationalImpact, ProblemSpec, ReplayPolicy, RiskPolicy,
    ValueDomain,
};
use serde::Serialize;
use std::collections::BTreeMap;

/// Transform an architecture's facets into a `pse-traverse` [`ProblemSpec`].
/// Components and edges alike become Boolean dimensions; every `depends_on`
/// relation becomes a hard constraint coupling foundation → dependent.
pub fn architecture_problem_spec(label: &str, facets: &[WishFacet]) -> ProblemSpec {
    let mut dimensions: Vec<DimensionSpec> = facets
        .iter()
        .map(|f| DimensionSpec {
            id: f.key.clone(),
            label: format!("{:?} {}", f.kind, f.key),
            kind: DimensionKind::Boolean,
            values: ValueDomain::Boolean,
            required: true,
            source: DimensionSource::User,
        })
        .collect();
    dimensions.sort_by(|a, b| a.id.cmp(&b.id));
    dimensions.dedup_by(|a, b| a.id == b.id);

    let mut constraints: Vec<ConstraintSpec> = Vec::new();
    for g in facets {
        for f in facets {
            if depends_on(g, f) {
                constraints.push(ConstraintSpec {
                    id: format!("{} -> {}", f.key, g.key),
                    kind: ConstraintKind::Hard,
                    predicate: "depends_on".into(),
                    weight: 1.0,
                    dimensions: vec![f.key.clone(), g.key.clone()],
                });
            }
        }
    }
    constraints.sort_by(|a, b| a.id.cmp(&b.id));
    constraints.dedup_by(|a, b| a.id == b.id);

    ProblemSpec {
        id: format!("arch.{}", label.replace(char::is_whitespace, "_")),
        title: label.to_string(),
        objective: "realize the architecture".into(),
        domain: "software-architecture".into(),
        inputs: Vec::new(),
        constraints,
        dimensions,
        desired_outputs: Vec::new(),
        risk_policy: RiskPolicy::default(),
        replay: ReplayPolicy::default(),
        metadata: BTreeMap::new(),
    }
}

/// One step of the rendered collapse plan — a kind plus the facets it targets.
#[derive(Debug, Clone, Serialize)]
pub struct PlanStepView {
    pub kind: String,
    pub targets: Vec<String>,
}

/// A facet that is formally wished but has no admissible operational path.
#[derive(Debug, Clone, Serialize)]
pub struct ExcisionView {
    pub facet: String,
    pub reason: String,
    pub impact: String,
}

/// The collapse plan, rendered to kosmo's vocabulary.
#[derive(Debug, Clone, Serialize)]
pub struct CollapsePlanView {
    pub steps: Vec<PlanStepView>,
    pub excisions: Vec<ExcisionView>,
    pub expected_reduction: f64,
}

fn step_word(kind: CollapseStepKind) -> &'static str {
    match kind {
        CollapseStepKind::ResolveDimension => "resolve",
        CollapseStepKind::ApplyConstraint => "wire",
        CollapseStepKind::SplitComponent => "split",
        CollapseStepKind::MergeComponent => "merge",
        CollapseStepKind::DetectPathExcision => "scan",
        CollapseStepKind::MigrateCarrier => "migrate",
        CollapseStepKind::VerifyCandidate => "verify",
        CollapseStepKind::CommitCandidate => "commit",
    }
}

fn impact_word(i: OperationalImpact) -> &'static str {
    match i {
        OperationalImpact::Low => "low",
        OperationalImpact::Medium => "medium",
        OperationalImpact::High => "high",
        OperationalImpact::Critical => "critical",
    }
}

/// Run the deterministic `pse-traverse` kernel over an architecture and return
/// the collapse plan + excisions, mapped back into kosmo's vocabulary.
pub fn collapse_plan_view(label: &str, facets: &[WishFacet]) -> Result<CollapsePlanView, String> {
    let spec = architecture_problem_spec(label, facets);
    let cube = DefaultFieldCubeBuilder
        .build(&spec)
        .map_err(|e| format!("field cube: {e}"))?;
    let graph = DoFGraph::from_field_cube(&cube);
    let excisions = detect_path_excision(&cube);
    let plan = DefaultCollapsePlanner.plan(&cube, &graph, &excisions);

    let steps = plan
        .steps
        .iter()
        .map(|s| {
            // A Boolean dimension contributes its key once per formal value; the
            // user wants the facet once, so dedup targets within a step.
            let mut targets: Vec<String> = s.target_nodes.iter().map(|n| n.0.clone()).collect();
            targets.dedup();
            PlanStepView {
                kind: step_word(s.kind).to_string(),
                targets,
            }
        })
        .collect();
    // One excision line per facet (not per Boolean formal value).
    let mut excisions: Vec<ExcisionView> = excisions
        .iter()
        .map(|e| ExcisionView {
            facet: e.dimension_id.clone(),
            reason: e.reason.clone(),
            impact: impact_word(e.operational_impact).to_string(),
        })
        .collect();
    excisions.dedup_by(|a, b| a.facet == b.facet);

    Ok(CollapsePlanView {
        steps,
        excisions,
        expected_reduction: plan.expected_reduction,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::WishFacetKind;

    fn facet(kind: WishFacetKind, key: &str) -> WishFacet {
        WishFacet::new(kind, key)
    }

    #[test]
    fn a_wired_hierarchy_yields_a_foundations_first_plan_no_excision() {
        // crate api ◁ module api::core ◁ symbol api::core::boot — a connected
        // hierarchy: every facet has a path, so nothing is excised, and the plan
        // resolves the foundation (the crate) before what stands on it.
        let facets = vec![
            facet(WishFacetKind::Crate, "api"),
            facet(WishFacetKind::Module, "api::core"),
            facet(WishFacetKind::Symbol, "api::core::boot"),
        ];
        let view = collapse_plan_view("hierarchy", &facets).expect("plan");
        assert!(
            view.excisions.is_empty(),
            "a connected hierarchy excises nothing: {:?}",
            view.excisions
        );
        // The crate is resolved before the symbol that stands on it.
        let resolve_order: Vec<&str> = view
            .steps
            .iter()
            .filter(|s| s.kind == "resolve")
            .flat_map(|s| s.targets.iter().map(|t| t.as_str()))
            .collect();
        let pos = |k: &str| resolve_order.iter().position(|t| *t == k);
        if let (Some(crate_at), Some(sym_at)) = (pos("api"), pos("api::core::boot")) {
            assert!(crate_at < sym_at, "foundation first: {resolve_order:?}");
        }
    }

    #[test]
    fn a_disconnected_component_is_excised() {
        // Two crates, no edges between them — a heap. Each has no admissible
        // path, so pse-traverse formally excises them (the operational cousin of
        // brick 47's "a heap, not an architecture").
        let facets = vec![
            facet(WishFacetKind::Crate, "api"),
            facet(WishFacetKind::Crate, "logging"),
        ];
        let view = collapse_plan_view("heap", &facets).expect("plan");
        assert!(
            !view.excisions.is_empty(),
            "disconnected components are operationally unreachable"
        );
        assert!(
            view.excisions.iter().any(|e| e.reason.contains("no_admissible_path")),
            "the reason is the missing path: {:?}",
            view.excisions
        );
    }
}

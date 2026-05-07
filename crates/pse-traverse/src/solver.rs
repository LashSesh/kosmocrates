//! Solver trait and reference implementations — Phase 5: Agent Harness.
//!
//! A `Solver` is the exchangeable solution operator that, given a `FieldCube`,
//! a `CollapsePlan` and a `SolverContext`, produces a list of `Candidate`s.
//! Every solver in the kernel must be deterministic and free of network I/O.
//!
//! Shipped implementations:
//! - [`NullSolver`]       — returns no candidates; exercises the gate-fail path.
//! - [`TemplateSolver`]   — assigns the first admissible value to every required
//!                          dimension; deterministic, zero configuration.
//!
//! The [`OracleSolver`] marker trait is defined but **not** implemented here.
//! Network-backed oracles are explicitly outside the kernel boundary (see
//! `NonGoals` in the spec, §5).
//!
//! [`ProjectedContext`] is the context-projection surface for future LLM-backed
//! solvers: a minimal, serialisable view of the problem that an oracle can act
//! on without direct access to engine internals.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::excision::PathExcision;
use crate::field_cube::FieldCube;
use crate::gate::Candidate;
use crate::plan::CollapsePlan;
use crate::spec::{ConstraintKind, ValueDomain};
use crate::Result;

// ─────────────────────────────────────────────────────────────────────────────
// SolverContext
// ─────────────────────────────────────────────────────────────────────────────

/// Contextual projection for solvers. Passed through from
/// `TraversalRunDescriptor`; this is the minimal view a solver (including a
/// future LLM solver) needs to act without raw access to the full engine state.
///
/// `allowed_oracle` mirrors `RiskPolicy::allow_oracle`. Solver impls MUST check
/// this flag before invoking any non-deterministic or network-backed oracle.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SolverContext {
    pub run_id: String,
    pub problem_spec_hash: String,
    pub seed: Option<u64>,
    pub allowed_oracle: bool,
    pub metadata: BTreeMap<String, String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Solver trait
// ─────────────────────────────────────────────────────────────────────────────

/// Core solver contract.
///
/// Implementations MUST be:
/// - Deterministic given the same `cube`, `plan` and `ctx`.
/// - Free of network I/O (kernel boundary).
/// - Free of non-`BTreeMap` keyed collections in any canonical output.
pub trait Solver {
    fn solve(
        &self,
        cube: &FieldCube,
        plan: &CollapsePlan,
        ctx: &SolverContext,
    ) -> Result<Vec<Candidate>>;
}

// ─────────────────────────────────────────────────────────────────────────────
// NullSolver
// ─────────────────────────────────────────────────────────────────────────────

/// Produces no candidates. Useful for gate-fail / no-commit path tests and as
/// a safe default when no solver is configured.
pub struct NullSolver;

impl Solver for NullSolver {
    fn solve(
        &self,
        _cube: &FieldCube,
        _plan: &CollapsePlan,
        _ctx: &SolverContext,
    ) -> Result<Vec<Candidate>> {
        Ok(Vec::new())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TemplateSolver
// ─────────────────────────────────────────────────────────────────────────────

/// Deterministic baseline solver.
///
/// For each required dimension (processed in alphabetical order for
/// determinism) assigns the first admissible value from its `ValueDomain` and
/// claims all constraints that couple that dimension. Two runs on the same cube
/// and context always produce byte-identical candidates.
pub struct TemplateSolver;

impl Solver for TemplateSolver {
    fn solve(
        &self,
        cube: &FieldCube,
        _plan: &CollapsePlan,
        _ctx: &SolverContext,
    ) -> Result<Vec<Candidate>> {
        let mut candidates = Vec::new();
        // BTreeMap iteration is alphabetical → determinism guaranteed.
        for (dim_id, dim) in &cube.dimensions {
            if !dim.required {
                continue;
            }
            let value = first_value(&dim.values);
            // Sort claimed constraints for stable content addresses.
            let mut claimed: Vec<String> = cube
                .constraints
                .values()
                .filter(|c| c.kind == ConstraintKind::Hard && c.dimensions.contains(dim_id))
                .map(|c| c.id.clone())
                .collect();
            claimed.sort();
            let candidate = Candidate {
                id: String::new(),
                field_cube_id: cube.id.clone(),
                assignments: {
                    let mut m = BTreeMap::new();
                    m.insert(dim_id.clone(), value);
                    m
                },
                claimed_satisfies: claimed,
                payloads: vec![dim_id.as_bytes().to_vec()],
                provenance: "template-solver-v1".into(),
            }
            .assign_id("cand")?;
            candidates.push(candidate);
        }
        Ok(candidates)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// OracleSolver (marker trait — no network impl in the kernel)
// ─────────────────────────────────────────────────────────────────────────────

/// Marker trait for oracle-backed solvers.
///
/// An oracle solver may call external tools or LLM APIs.  It is
/// **intentionally not implemented** inside the kernel.  Any implementation
/// must:
/// 1. Be feature-gated.
/// 2. Check `ctx.allowed_oracle` before executing.
/// 3. Never emit `SemanticCrystal`s directly — all commits must still go
///    through `pse_core::macro_step` via [`crate::bridge::PseMacroStepCommitter`].
pub trait OracleSolver: Solver {
    /// Stable identifier for this oracle (e.g. `"openai-gpt4o-v1"`).
    fn oracle_id(&self) -> &str;
    /// True if the oracle makes outbound network calls.
    fn is_network_backed(&self) -> bool;
}

// ─────────────────────────────────────────────────────────────────────────────
// ProjectedContext — LLM-agent context projection
// ─────────────────────────────────────────────────────────────────────────────

/// Minimal, serialisable projection of the problem state for future LLM-backed
/// solvers.
///
/// `ProjectedContext::project` converts a `FieldCube`, path excisions and a
/// `SolverContext` into a structured summary that an oracle can process without
/// raw access to engine internals. The projection is deterministic and
/// content-addressable via [`crate::canonical::content_address`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProjectedContext {
    pub run_id: String,
    pub objective: String,
    /// `"<dim_id>: <label> [required]"` entries, sorted.
    pub required_dimensions: Vec<String>,
    /// `"<constraint_id>: <predicate>"` entries for hard constraints, sorted.
    pub hard_constraints: Vec<String>,
    /// `"<dim_id>/<formal_value> (<reason>)"` entries, sorted.
    pub path_excisions: Vec<String>,
    pub seed: Option<u64>,
}

impl ProjectedContext {
    /// Build a `ProjectedContext` from the engine state.
    ///
    /// The projection is deterministic: all vectors are sorted before storage.
    pub fn project(
        ctx: &SolverContext,
        cube: &FieldCube,
        excisions: &[PathExcision],
        objective: &str,
    ) -> Self {
        let mut required_dimensions: Vec<String> = cube
            .dimensions
            .values()
            .filter(|d| d.required)
            .map(|d| format!("{}: {}", d.id, d.label))
            .collect();
        required_dimensions.sort();

        let mut hard_constraints: Vec<String> = cube
            .constraints
            .values()
            .filter(|c| c.kind == ConstraintKind::Hard)
            .map(|c| format!("{}: {}", c.id, c.predicate))
            .collect();
        hard_constraints.sort();

        let mut path_excisions: Vec<String> = excisions
            .iter()
            .map(|e| format!("{}/{} ({})", e.dimension_id, e.formal_value, e.reason))
            .collect();
        path_excisions.sort();

        ProjectedContext {
            run_id: ctx.run_id.clone(),
            objective: objective.to_string(),
            required_dimensions,
            hard_constraints,
            path_excisions,
            seed: ctx.seed,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn first_value(domain: &ValueDomain) -> String {
    match domain {
        ValueDomain::Boolean => "true".to_string(),
        ValueDomain::Enum(v) => v.first().cloned().unwrap_or_default(),
        ValueDomain::Range { min, .. } => format!("{}", min),
        ValueDomain::Tree(t) => t.first().map(|v| v.label.clone()).unwrap_or_default(),
        ValueDomain::External { resolver } => format!("ext:{}", resolver),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::content_address;
    use crate::dof::DoFGraph;
    use crate::excision::detect_path_excision;
    use crate::field_cube::{DefaultFieldCubeBuilder, FieldCubeBuilder};
    use crate::plan::{CollapsePlanner, DefaultCollapsePlanner};
    use crate::spec::{
        ConstraintSpec, DimensionKind, DimensionSource, DimensionSpec, OutputSpec, ProblemSpec,
        ReplayPolicy, RiskPolicy, ValueDomain,
    };

    fn minimal_spec() -> ProblemSpec {
        ProblemSpec {
            id: "p.solver".into(),
            title: "T".into(),
            objective: "test objective".into(),
            domain: "d".into(),
            inputs: vec![],
            constraints: vec![ConstraintSpec {
                id: "c.h".into(),
                kind: ConstraintKind::Hard,
                predicate: "always_true".into(),
                weight: 1.0,
                dimensions: vec!["d.x".into()],
            }],
            dimensions: vec![DimensionSpec {
                id: "d.x".into(),
                label: "X".into(),
                kind: DimensionKind::Boolean,
                values: ValueDomain::Boolean,
                required: true,
                source: DimensionSource::User,
            }],
            desired_outputs: vec![OutputSpec {
                id: "o.x".into(),
                kind: "y".into(),
            }],
            risk_policy: RiskPolicy::default(),
            replay: ReplayPolicy {
                seed: Some(42),
                canonical: true,
            },
            metadata: BTreeMap::new(),
        }
    }

    fn ctx() -> SolverContext {
        SolverContext {
            run_id: "run.test.0".into(),
            problem_spec_hash: "abc123".into(),
            seed: Some(42),
            allowed_oracle: false,
            metadata: BTreeMap::new(),
        }
    }

    fn setup() -> (FieldCube, CollapsePlan) {
        let spec = minimal_spec();
        let cube = DefaultFieldCubeBuilder.build(&spec).unwrap();
        let graph = DoFGraph::from_field_cube(&cube);
        let exc = detect_path_excision(&cube);
        let plan = DefaultCollapsePlanner.plan(&cube, &graph, &exc);
        (cube, plan)
    }

    #[test]
    fn null_solver_returns_empty() {
        let (cube, plan) = setup();
        let candidates = NullSolver.solve(&cube, &plan, &ctx()).unwrap();
        assert!(candidates.is_empty());
    }

    #[test]
    fn template_solver_assigns_all_required_dims() {
        let (cube, plan) = setup();
        let candidates = TemplateSolver.solve(&cube, &plan, &ctx()).unwrap();
        // One required dimension → one candidate
        assert_eq!(candidates.len(), 1);
        assert!(
            candidates[0].assignments.contains_key("d.x"),
            "expected d.x in {:?}",
            candidates[0].assignments
        );
    }

    #[test]
    fn template_solver_is_deterministic() {
        let (cube, plan) = setup();
        let c1 = TemplateSolver.solve(&cube, &plan, &ctx()).unwrap();
        let c2 = TemplateSolver.solve(&cube, &plan, &ctx()).unwrap();
        assert_eq!(c1, c2);
    }

    #[test]
    fn template_solver_candidates_are_content_addressed() {
        let (cube, plan) = setup();
        let candidates = TemplateSolver.solve(&cube, &plan, &ctx()).unwrap();
        for c in &candidates {
            // id is non-empty (assigned by assign_id)
            assert!(!c.id.is_empty());
            // Stable across re-derives
            let h1 = content_address(c).unwrap();
            let h2 = content_address(c).unwrap();
            assert_eq!(h1, h2);
        }
    }

    #[test]
    fn solver_context_serialises_stably() {
        let c = ctx();
        let h1 = content_address(&c).unwrap();
        let h2 = content_address(&c).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn projected_context_contains_required_dims_and_constraints() {
        let spec = minimal_spec();
        let cube = DefaultFieldCubeBuilder.build(&spec).unwrap();
        let exc = detect_path_excision(&cube);
        let proj = ProjectedContext::project(&ctx(), &cube, &exc, &spec.objective);
        assert!(!proj.required_dimensions.is_empty(), "no required dims");
        assert!(!proj.hard_constraints.is_empty(), "no hard constraints");
        assert_eq!(proj.run_id, "run.test.0");
        assert_eq!(proj.objective, "test objective");
    }

    #[test]
    fn projected_context_is_deterministic() {
        let spec = minimal_spec();
        let cube = DefaultFieldCubeBuilder.build(&spec).unwrap();
        let exc = detect_path_excision(&cube);
        let p1 = ProjectedContext::project(&ctx(), &cube, &exc, &spec.objective);
        let p2 = ProjectedContext::project(&ctx(), &cube, &exc, &spec.objective);
        assert_eq!(p1, p2);
        assert_eq!(content_address(&p1).unwrap(), content_address(&p2).unwrap());
    }
}

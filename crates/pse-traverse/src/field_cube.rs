//! `FieldCube` — the executable form of the extended Hypercube.
//!
//! Carries dimensions, constraints, couplings, paths, carriers, evidence
//! and a topology summary. All keyed collections are `BTreeMap`; all
//! vectors are sorted before serialisation by their stable IDs.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::spec::{ConstraintSpec, DimensionSpec, ProblemSpec, ValueDomain};
use crate::{Result, TraverseError};

/// A coupling between two constraints / dimensions in the field cube.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Coupling {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub strength: f64,
}

/// A path through the cube — a candidate trajectory from start to a
/// dimension target.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PathSpec {
    pub id: String,
    pub from: String,
    pub to: String,
    /// Constraint IDs gating this path.
    pub gating_constraints: Vec<String>,
    pub admissible: bool,
}

/// A carrier — an operative processing mode (Analysis, Decomposition,
/// Solve, Verify, Commit, Coagula). Not a physical object in the MVP;
/// a logical channel that prevents drift between unrelated steps.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CarrierState {
    pub id: String,
    pub kind: CarrierKind,
    pub load: f64,
    pub saturation: f64,
    pub friction: f64,
    pub shock: f64,
    pub coherence: f64,
    pub status: CarrierStatus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CarrierKind {
    Analysis,
    Decomposition,
    Solve,
    Verify,
    Commit,
    Coagula,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CarrierStatus {
    Idle,
    Active,
    Saturated,
    Migrating,
    Frozen,
}

/// Reference to a piece of evidence by content address.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceRef {
    /// 64-char hex SHA-256.
    pub address: String,
    /// Free-form schema label so a verifier knows what to recompute.
    pub schema: String,
}

/// Lightweight topology summary. Computed locally in the MVP; will be
/// upgraded to use `pse_topology` (Laplacian, Fiedler, Betti) in a
/// later phase.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct TopologySummary {
    pub node_count: u64,
    pub edge_count: u64,
    pub component_count: u64,
    /// 1 if at least one undirected cycle is present, else 0. Cheap proxy.
    pub has_cycle: u8,
    /// Mean degree as a coarse centrality proxy.
    pub mean_degree: f64,
}

/// The field cube itself.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FieldCube {
    pub id: String,
    pub problem_id: String,
    pub dimensions: BTreeMap<String, DimensionSpec>,
    pub constraints: BTreeMap<String, ConstraintSpec>,
    pub couplings: Vec<Coupling>,
    pub paths: Vec<PathSpec>,
    pub carriers: Vec<CarrierState>,
    pub evidence: Vec<EvidenceRef>,
    pub topology: TopologySummary,
    pub created_by: String,
}

/// Trait for field-cube builders. The MVP ships
/// [`DefaultFieldCubeBuilder`]; future phases can plug in
/// domain-specific builders.
pub trait FieldCubeBuilder {
    fn build(&self, spec: &ProblemSpec) -> Result<FieldCube>;
}

/// Reference builder. Deterministic, sorts everything, derives couplings
/// from explicit constraint→dimension references, initialises six
/// carriers (one per `CarrierKind`).
pub struct DefaultFieldCubeBuilder;

impl FieldCubeBuilder for DefaultFieldCubeBuilder {
    fn build(&self, spec: &ProblemSpec) -> Result<FieldCube> {
        if spec.id.trim().is_empty() {
            return Err(TraverseError::InvalidSpec("ProblemSpec.id is empty".into()));
        }
        let mut dimensions = BTreeMap::new();
        for d in &spec.dimensions {
            if dimensions.contains_key(&d.id) {
                return Err(TraverseError::InvalidSpec(format!(
                    "duplicate dimension id: {}",
                    d.id
                )));
            }
            // Cheap sanity: enum domains must be non-empty.
            if let ValueDomain::Enum(v) = &d.values {
                if v.is_empty() {
                    return Err(TraverseError::InvalidSpec(format!(
                        "dimension {} has empty enum domain",
                        d.id
                    )));
                }
            }
            dimensions.insert(d.id.clone(), d.clone());
        }
        let mut constraints = BTreeMap::new();
        for c in &spec.constraints {
            if constraints.contains_key(&c.id) {
                return Err(TraverseError::InvalidSpec(format!(
                    "duplicate constraint id: {}",
                    c.id
                )));
            }
            constraints.insert(c.id.clone(), c.clone());
        }

        // Derive couplings from explicit constraint→dimension refs.
        let mut couplings: Vec<Coupling> = Vec::new();
        for c in spec.constraints.iter() {
            for d in &c.dimensions {
                couplings.push(Coupling {
                    from: c.id.clone(),
                    to: d.clone(),
                    kind: format!("{:?}", c.kind).to_lowercase(),
                    strength: c.weight,
                });
            }
        }
        couplings.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));

        // Default paths: from a synthetic "start" pseudo-node to every required
        // dimension. Path is admissible iff at least one of its gating
        // constraints exists in `constraints` (operative-option proxy).
        let mut paths: Vec<PathSpec> = Vec::new();
        for d in spec.dimensions.iter().filter(|d| d.required) {
            let mut gating: Vec<String> = spec
                .constraints
                .iter()
                .filter(|c| c.dimensions.contains(&d.id))
                .map(|c| c.id.clone())
                .collect();
            gating.sort();
            let admissible = !gating.is_empty();
            paths.push(PathSpec {
                id: format!("p.start->{}", d.id),
                from: "start".into(),
                to: d.id.clone(),
                gating_constraints: gating,
                admissible,
            });
        }
        paths.sort_by(|a, b| a.id.cmp(&b.id));

        let mut carriers: Vec<CarrierState> = vec![
            CarrierKind::Analysis,
            CarrierKind::Decomposition,
            CarrierKind::Solve,
            CarrierKind::Verify,
            CarrierKind::Commit,
            CarrierKind::Coagula,
        ]
        .into_iter()
        .map(|k| CarrierState {
            id: format!("{:?}", k).to_lowercase(),
            kind: k,
            load: 0.0,
            saturation: 0.0,
            friction: 0.0,
            shock: 0.0,
            coherence: 1.0,
            status: CarrierStatus::Idle,
        })
        .collect();
        carriers.sort_by(|a, b| a.id.cmp(&b.id));

        let topology = compute_topology(&dimensions, &constraints, &couplings);

        let cube = FieldCube {
            id: format!("fc.{}", spec.id),
            problem_id: spec.id.clone(),
            dimensions,
            constraints,
            couplings,
            paths,
            carriers,
            evidence: Vec::new(),
            topology,
            created_by: "DefaultFieldCubeBuilder".into(),
        };
        Ok(cube)
    }
}

fn compute_topology(
    dimensions: &BTreeMap<String, DimensionSpec>,
    constraints: &BTreeMap<String, ConstraintSpec>,
    couplings: &[Coupling],
) -> TopologySummary {
    let nodes: BTreeSet<&str> = dimensions
        .keys()
        .map(|s| s.as_str())
        .chain(constraints.keys().map(|s| s.as_str()))
        .collect();
    let n = nodes.len() as u64;
    let m = couplings.len() as u64;

    // Component count via union-find on coupling edges.
    let mut idx: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, n) in nodes.iter().enumerate() {
        idx.insert(*n, i);
    }
    let mut parent: Vec<usize> = (0..nodes.len()).collect();
    fn find(p: &mut [usize], x: usize) -> usize {
        if p[x] == x {
            x
        } else {
            let r = find(p, p[x]);
            p[x] = r;
            r
        }
    }
    for c in couplings {
        if let (Some(&a), Some(&b)) = (idx.get(c.from.as_str()), idx.get(c.to.as_str())) {
            let ra = find(&mut parent, a);
            let rb = find(&mut parent, b);
            if ra != rb {
                parent[ra] = rb;
            }
        }
    }
    let comp_roots: BTreeSet<usize> = (0..parent.len()).map(|i| find(&mut parent, i)).collect();
    let component_count = if n > 0 { comp_roots.len() as u64 } else { 0 };

    // Cycle proxy: m > n - components ⇒ at least one cycle in the bipartite graph.
    let has_cycle: u8 = if n > 0 && m + component_count > n {
        1
    } else {
        0
    };

    let mut deg: BTreeMap<&str, u64> = BTreeMap::new();
    for c in couplings {
        *deg.entry(c.from.as_str()).or_insert(0) += 1;
        *deg.entry(c.to.as_str()).or_insert(0) += 1;
    }
    let mean_degree = if n > 0 {
        deg.values().sum::<u64>() as f64 / n as f64
    } else {
        0.0
    };

    TopologySummary {
        node_count: n,
        edge_count: m,
        component_count,
        has_cycle,
        mean_degree,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::content_address;
    use crate::spec::{
        ConstraintKind, ConstraintSpec, DimensionKind, DimensionSource, OutputSpec, ReplayPolicy,
        RiskPolicy,
    };

    fn minimal_spec() -> ProblemSpec {
        ProblemSpec {
            id: "p.minimal".into(),
            title: "Min".into(),
            objective: "obj".into(),
            domain: "d".into(),
            inputs: vec![],
            constraints: vec![ConstraintSpec {
                id: "c.x".into(),
                kind: ConstraintKind::Hard,
                predicate: "true".into(),
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
                id: "o.x".into(),
                kind: "x".into(),
            }],
            risk_policy: RiskPolicy::default(),
            replay: ReplayPolicy {
                seed: Some(7),
                canonical: true,
            },
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn fieldcube_builds_from_minimal_spec() {
        let cube = DefaultFieldCubeBuilder.build(&minimal_spec()).unwrap();
        assert_eq!(cube.problem_id, "p.minimal");
        assert!(cube.dimensions.contains_key("d.layout"));
        assert!(cube.constraints.contains_key("c.x"));
        assert_eq!(cube.couplings.len(), 1);
        assert_eq!(cube.carriers.len(), 6);
        assert_eq!(cube.topology.node_count, 2);
        assert_eq!(cube.topology.edge_count, 1);
    }

    #[test]
    fn fieldcube_address_stable_across_runs() {
        let cube1 = DefaultFieldCubeBuilder.build(&minimal_spec()).unwrap();
        let cube2 = DefaultFieldCubeBuilder.build(&minimal_spec()).unwrap();
        assert_eq!(
            content_address(&cube1).unwrap(),
            content_address(&cube2).unwrap()
        );
    }

    #[test]
    fn fieldcube_rejects_duplicate_dimension_id() {
        let mut s = minimal_spec();
        s.dimensions.push(s.dimensions[0].clone());
        let r = DefaultFieldCubeBuilder.build(&s);
        assert!(matches!(r, Err(TraverseError::InvalidSpec(_))));
    }

    #[test]
    fn fieldcube_rejects_empty_enum_domain() {
        let mut s = minimal_spec();
        s.dimensions[0].values = ValueDomain::Enum(vec![]);
        let r = DefaultFieldCubeBuilder.build(&s);
        assert!(matches!(r, Err(TraverseError::InvalidSpec(_))));
    }
}

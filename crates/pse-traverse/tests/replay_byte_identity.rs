//! End-to-end determinism check.
//!
//! Loads `examples/problem_minimal.json`, builds the FieldCube /
//! DoFGraph / CollapsePlan twice, and asserts byte-for-byte canonical
//! identity of every artefact.

use pse_traverse::canonical::canonical_bytes;
use pse_traverse::dof::DoFGraph;
use pse_traverse::excision::detect_path_excision;
use pse_traverse::field_cube::{DefaultFieldCubeBuilder, FieldCubeBuilder};
use pse_traverse::plan::{CollapsePlanner, DefaultCollapsePlanner};
use pse_traverse::spec::ProblemSpec;

const MINIMAL: &str = include_str!("../examples/problem_minimal.json");

fn pipeline(spec: &ProblemSpec) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let cube = DefaultFieldCubeBuilder.build(spec).expect("build");
    let graph = DoFGraph::from_field_cube(&cube);
    let exc = detect_path_excision(&cube);
    let plan = DefaultCollapsePlanner.plan(&cube, &graph, &exc);
    (
        canonical_bytes(&cube).unwrap(),
        canonical_bytes(&graph).unwrap(),
        canonical_bytes(&exc).unwrap(),
        canonical_bytes(&plan).unwrap(),
    )
}

#[test]
fn minimal_problem_is_byte_identical_across_two_runs() {
    let spec: ProblemSpec = serde_json::from_str(MINIMAL).unwrap();
    let (cube_a, graph_a, exc_a, plan_a) = pipeline(&spec);
    let (cube_b, graph_b, exc_b, plan_b) = pipeline(&spec);
    assert_eq!(cube_a, cube_b, "field cube canonical bytes differ");
    assert_eq!(graph_a, graph_b, "dof graph canonical bytes differ");
    assert_eq!(exc_a, exc_b, "excisions canonical bytes differ");
    assert_eq!(plan_a, plan_b, "collapse plan canonical bytes differ");
}

#[test]
fn minimal_problem_passes_path_excision() {
    let spec: ProblemSpec = serde_json::from_str(MINIMAL).unwrap();
    let cube = DefaultFieldCubeBuilder.build(&spec).unwrap();
    let exc = detect_path_excision(&cube);
    // d.crate_layout is referenced by both hard constraints → no excision.
    assert!(exc.is_empty(), "unexpected excisions: {:?}", exc);
}

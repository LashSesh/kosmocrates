//! Run 42 — the behavioural lattice (the "maximum"): `--behaviour` combines real
//! runnable functions by *executed* composition, dedups by observational
//! equality, saturates to the generated transformation monoid, and bridges back
//! to structure — synonyms (different code, one behaviour) and false friends
//! (similar structure, divergent behaviour). Workspace-independent.

use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// `--behaviour` ignores the workspace; any path serves.
fn anywhere() -> std::path::PathBuf {
    std::env::temp_dir()
}

fn behaviour_json(extra: &[&str]) -> serde_json::Value {
    let path = anywhere();
    let mut args: Vec<&str> = vec!["--behaviour", "--json"];
    args.extend_from_slice(extra);
    let p = path.to_str().unwrap();
    args.push(p);
    let out = kosmo_run().args(&args).output().expect("spawn kosmo-run");
    serde_json::from_slice(&out.stdout).expect("valid JSON")
}

#[test]
fn behaviour_saturates_to_a_closed_monoid() {
    let v = behaviour_json(&[]);
    assert_eq!(v["domain"].as_u64(), Some(12));
    assert_eq!(v["generators"].as_u64(), Some(4));
    let monoid = v["monoid"].as_u64().unwrap();
    assert!(monoid >= 4, "the monoid contains at least its generators");
    assert_eq!(v["fixpoint"], true, "composition reached a fixpoint (closed)");
    assert!(
        v["explored"].as_u64().unwrap() > monoid,
        "far more compositions were executed than distinct behaviours found"
    );
}

#[test]
fn behaviour_bridge_quantifies_the_structural_gap() {
    let v = behaviour_json(&[]);
    // Synonyms: different code collapsing to one behaviour — the proxy under-merges.
    assert!(
        v["synonyms"].as_u64().unwrap() > 0,
        "executed dedup collapses distinct code to a shared behaviour"
    );
    // False friends: structurally-alike yet behaviourally-distinct pairs — the
    // proxy over-merges. The monoid must contain at least one such pair.
    assert!(
        v["false_friends"].as_u64().unwrap() >= 1,
        "the structural proxy mistakes some distinct behaviours for kin"
    );
    assert!(v["pairs"].as_u64().unwrap() >= v["false_friends"].as_u64().unwrap());
}

#[test]
fn behaviour_certify_rejects_constants_and_shrinks_the_monoid() {
    let plain = behaviour_json(&[]);
    let certified = behaviour_json(&["--certify"]);
    assert_eq!(certified["certify"], true);
    assert!(
        certified["rejected_constant"].as_u64().unwrap() >= 1,
        "the constant generator is turned away"
    );
    // Behavioural validity is NOT closed under composition: arming the gate
    // removes the constant *and* every offspring that collapses through it.
    assert!(
        certified["monoid"].as_u64().unwrap() < plain["monoid"].as_u64().unwrap(),
        "the informativeness gate yields a strictly smaller lattice: \
         certified={certified} plain={plain}"
    );
}

#[test]
fn behaviour_is_deterministic() {
    let a = behaviour_json(&[]);
    let b = behaviour_json(&[]);
    for k in ["monoid", "explored", "synonyms", "false_friends"] {
        assert_eq!(a[k], b[k], "the executed lattice is deterministic at {k}");
    }
}

#[test]
fn behaviour_is_one_door_per_run() {
    let path = anywhere();
    let out = kosmo_run()
        .args(["--behaviour", "--codematrix", path.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success(), "two organ doors in one run is rejected");
}

#[test]
fn behaviour_human_render_shows_executed_tables() {
    let path = anywhere();
    let out = kosmo_run()
        .args(["--behaviour", "--no-color", path.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("executed lattice"), "names the lattice: {stdout}");
    assert!(stdout.contains("synonyms"), "reports the bridge: {stdout}");
    assert!(stdout.contains("false friends"), "reports the bridge: {stdout}");
    // The identity behaviour's table is the domain in order.
    assert!(
        stdout.contains("[0 1 2 3 4 5 6 7 8 9 10 11]"),
        "shows an executed truth table: {stdout}"
    );
}

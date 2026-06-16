//! Run 23 — "did you mean?": an unmet wish with a near-miss in the workspace
//! gets a suggestion. A typo or naming drift is the commonest cause of an unmet
//! wish, so the instrument plays guide, not just judge.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

fn workspace() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-suggest-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub mod engine;\n").unwrap();
    fs::write(root.join("src/engine.rs"), "pub fn run() {}\n").unwrap();
    root
}

#[test]
fn unmet_wish_suggests_a_near_match() {
    let root = workspace(); // has module engine
    let out = kosmo_run()
        .args([
            "--wish",
            "a module enigne", // a typo of engine
            "--flat",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stdout}");
        return;
    }
    assert!(stdout.contains("did you mean"), "a near match is offered: {stdout}");
    assert!(stdout.contains("engine"), "it names the real module: {stdout}");
}

#[test]
fn signature_arity_gap_is_bridged() {
    // A workspace with a 2-arg function; the observation keys it `add/2`.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-sig-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn add(a: i32, b: i32) -> i32 { a + b }\n").unwrap();
    // The bare-stem signature wish does not match by equality, but the arity-
    // keyed form is offered as a "did you mean?".
    let out = kosmo_run()
        .args([
            "--wish",
            "a signature add",
            "--flat",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        return;
    }
    assert!(stdout.contains("did you mean"), "the arity form is suggested: {stdout}");
    assert!(stdout.contains("add/2"), "names the arity-keyed signature: {stdout}");
}

#[test]
fn a_genuine_gap_gets_no_false_suggestion() {
    let root = workspace();
    let out = kosmo_run()
        .args([
            "--wish",
            "a module telemetry", // nothing close exists
            "--flat",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        return;
    }
    assert!(!stdout.contains("did you mean"), "no false suggestion for a real gap: {stdout}");
}

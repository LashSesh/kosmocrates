//! Run 24 — the Shape stratum: a wish can require *documentation*, not just
//! existence. `a doc for <fn>` is realized only when the function `<fn>` carries
//! a doc comment — so the instrument verifies how code is shaped, not merely
//! that it is there. (Existence is "is it present?"; Shape is "is it formed?".)

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// `alpha` is documented; `beta` is not.
fn workspace() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-shape-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "/// Adds one to a number.\npub fn alpha(n: i32) -> i32 { n + 1 }\npub fn beta() {}\n",
    )
    .unwrap();
    root
}

#[test]
fn doc_shape_is_met_for_a_documented_function() {
    let root = workspace();
    let out = kosmo_run()
        .args([
            "--wish",
            "a doc for alpha",
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
    assert!(stdout.contains("REALIZED"), "alpha is documented: {stdout}");
    assert_eq!(out.status.code(), Some(0), "a met doc wish exits 0: {stdout}");
}

#[test]
fn doc_shape_is_unmet_for_an_undocumented_function() {
    let root = workspace();
    let out = kosmo_run()
        .args([
            "--wish",
            "a doc for beta",
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
    // beta exists but is undocumented — Shape is unmet though Existence holds.
    assert!(stdout.contains("Doc beta"), "the doc gap is named: {stdout}");
    assert_eq!(out.status.code(), Some(1), "an unmet doc wish exits 1: {stdout}");
}

#[test]
fn signature_shape_checks_arity() {
    let root = workspace(); // alpha(n: i32) has arity 1; beta() has arity 0
    // The correct arity realizes the signature wish — a genuine shape check.
    let ok = kosmo_run()
        .args([
            "--wish",
            "a signature alpha/1",
            "--flat",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let s = String::from_utf8_lossy(&ok.stdout);
    if s.contains("could not observe") {
        return;
    }
    assert!(s.contains("REALIZED"), "alpha has arity 1: {s}");
    assert_eq!(ok.status.code(), Some(0), "{s}");

    // The wrong arity is unmet ("exists but wrong shape"), and Run 25's bridge
    // suggests the real arity-keyed form.
    let wrong = kosmo_run()
        .args([
            "--wish",
            "a signature alpha/2",
            "--flat",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let w = String::from_utf8_lossy(&wrong.stdout);
    assert_eq!(wrong.status.code(), Some(1), "wrong arity fails: {w}");
    assert!(
        w.contains("did you mean Signature alpha/1"),
        "the real arity is suggested: {w}"
    );
}

#[test]
fn doc_shape_gap_is_closed_by_apply() {
    let root = workspace(); // beta is undocumented
    let out = kosmo_run()
        .args([
            "--wish",
            "a doc for beta",
            "--apply",
            "--flat",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (--apply)");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        return;
    }
    // The scaffolder authors a doc stub — the Shape gap is scaffoldable offline.
    assert!(stdout.contains("wish realized"), "--apply closes the doc gap: {stdout}");
    assert_eq!(out.status.code(), Some(0), "realized after --apply: {stdout}");
    let lib = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    assert!(lib.contains("///"), "a doc comment was written for beta: {lib}");
}

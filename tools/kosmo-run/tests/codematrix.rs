//! Run 37 — the holistic polyglot cube. `--codematrix` pools every per-source
//! 5D code-fingerprint (any recognized language, via `from_auto`) into ONE
//! language-blind centroid, plus a homogeneity scalar (mean pairwise resonance:
//! how unified the whole codebase's topology is). The operator's idea — "map
//! languages into hypercubes and compress them to one holistic" — made visible.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// A genuinely polyglot tree: Rust + Python + Go source in one directory.
fn polyglot_workspace() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-cm-{nanos}"));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("calc.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn mul(a: i32, b: i32) -> i32 { a * b }\n",
    )
    .unwrap();
    fs::write(
        root.join("greeter.py"),
        "def greet(name):\n    return \"hi \" + name\n\ndef bye(name):\n    return \"bye \" + name\n",
    )
    .unwrap();
    fs::write(
        root.join("util.go"),
        "package util\nfunc Square(x int) int { return x * x }\nfunc Cube(x int) int { return x * x * x }\n",
    )
    .unwrap();
    root
}

#[test]
fn codematrix_renders_a_holistic_polyglot_cube() {
    let root = polyglot_workspace();
    let out = kosmo_run()
        .args(["--codematrix", "--no-color", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("3 source"), "all three languages are fingerprinted: {stdout}");
    assert!(
        stdout.contains("holistic cube"),
        "the language-blind holistic cube is rendered: {stdout}"
    );
    assert!(stdout.contains("homogeneity"), "with a homogeneity scalar: {stdout}");
}

#[test]
fn codematrix_json_carries_the_holistic_reading() {
    let root = polyglot_workspace();
    let out = kosmo_run()
        .args(["--codematrix", "--json", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("valid JSON ({e}): {stdout}"));
    let holistic = &v["holistic"];
    assert!(holistic["axes"].is_array(), "holistic axes present: {stdout}");
    assert_eq!(holistic["axes"].as_array().map(|a| a.len()), Some(5), "five axes: {stdout}");
    assert!(holistic["homogeneity"].is_number(), "homogeneity present: {stdout}");
}

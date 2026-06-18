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
    assert!(
        stdout.contains("cross-language structural agreement"),
        "the language-independent cross-language equivalence is surfaced: {stdout}"
    );
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
    assert_eq!(v["cross_language"]["languages"].as_u64(), Some(3), "three languages: {stdout}");
    assert!(
        v["cross_language"]["agreement"].is_number(),
        "cross-language agreement present: {stdout}"
    );
}

/// A two-file workspace in two languages.
fn two_lang_workspace(f1: &str, c1: &str, f2: &str, c2: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-x2-{nanos}"));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(f1), c1).unwrap();
    fs::write(root.join(f2), c2).unwrap();
    root
}

fn codematrix_agreement(root: &std::path::Path) -> f64 {
    let out = kosmo_run()
        .args(["--codematrix", "--json", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    v["cross_language"]["agreement"]
        .as_f64()
        .expect("agreement present")
}

#[test]
fn cross_language_agreement_recognizes_equivalent_structure() {
    // The same shape (functions only) in Rust and Python — the language-
    // independent fingerprint lands them on the SAME point: agreement 1.0.
    let same = two_lang_workspace(
        "calc.rs",
        "pub fn a(x: i32) -> i32 { x }\npub fn b(x: i32) -> i32 { x }\n",
        "calc.py",
        "def a(x):\n    return x\ndef b(x):\n    return x\n",
    );
    assert_eq!(
        codematrix_agreement(&same),
        1.0,
        "equivalent structure across languages → agreement 1.0"
    );
    // Different shapes (functions vs classes) — strictly less than identical.
    let diff = two_lang_workspace(
        "calc.rs",
        "pub fn a(x: i32) -> i32 { x }\npub fn b(x: i32) -> i32 { x }\n",
        "types.py",
        "class A:\n    pass\nclass B:\n    pass\n",
    );
    assert!(
        codematrix_agreement(&diff) < 1.0,
        "differing structure → agreement below 1.0"
    );
}

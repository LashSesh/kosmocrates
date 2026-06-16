//! Run 28 — the Wiring stratum: a *typed contract*. `a contract f(T..)->R` is
//! met only when the function `f` has exactly that typed signature — so the
//! instrument verifies how things connect by type, not merely that a function
//! exists. "Exists but wrong types" is a real, distinct failure.
//!
//! (This corrects Run 26, which wrongly concluded the Wiring stratum was
//! unreachable — that was a wrong-prose-form error: `with signature ...`
//! mis-parses, but the canonical `a contract ...` form works read-only.)

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

fn workspace(lib: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-wiring-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), lib).unwrap();
    root
}

#[test]
fn typed_contract_is_met_for_the_right_signature() {
    let root = workspace("pub fn add(a: i32, b: i32) -> i32 { a + b }\n");
    let out = kosmo_run()
        .args([
            "--wish",
            "a contract add(i32,i32)->i32",
            "--flat",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let s = String::from_utf8_lossy(&out.stdout);
    if s.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {s}");
        return;
    }
    assert!(s.contains("REALIZED"), "the exact typed signature matches: {s}");
    assert_eq!(out.status.code(), Some(0), "a met contract exits 0: {s}");
}

#[test]
fn typed_contract_is_unmet_for_the_wrong_signature() {
    // `add` exists, but as (u8) -> u8 — wrong types and arity.
    let root = workspace("pub fn add(a: u8) -> u8 { a }\n");
    let out = kosmo_run()
        .args([
            "--wish",
            "a contract add(i32,i32)->i32",
            "--flat",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let s = String::from_utf8_lossy(&out.stdout);
    if s.contains("could not observe") {
        return;
    }
    assert!(
        s.contains("Contract add(i32,i32)->i32"),
        "the typed gap is named (exists but wrong types): {s}"
    );
    assert_eq!(out.status.code(), Some(1), "wrong types → unmet: {s}");
}

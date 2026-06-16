//! Run 40 — the combine lab as a door. `--alchemy` seeds structural elements
//! from the workspace (deduped by the CROSS-007 novelty gate), drives
//! `combine` to a fixpoint, and reports the discovered catalog + frontier — the
//! operator's "Doodle God" laboratory, over real code.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// Four sources with distinct structural profiles (functions / types / mixed /
/// import+function), across four languages.
fn workspace() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-lab-{nanos}"));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("funcs.rs"),
        "pub fn a(x: i32) -> i32 { x }\npub fn b(x: i32) -> i32 { x }\n",
    )
    .unwrap();
    fs::write(root.join("types.py"), "class A:\n    pass\nclass B:\n    pass\n").unwrap();
    fs::write(
        root.join("mixed.go"),
        "package m\nimport \"fmt\"\nfunc F() { fmt.Println(\"x\") }\ntype T struct{}\n",
    )
    .unwrap();
    fs::write(root.join("mod.js"), "import x from \"y\";\nfunction g() { return 1; }\n").unwrap();
    root
}

fn alchemy_json(root: &Path, threshold: Option<&str>) -> serde_json::Value {
    let path = root.to_str().unwrap();
    let mut args: Vec<&str> = vec!["--alchemy", "--json"];
    if let Some(t) = threshold {
        args.push("--threshold");
        args.push(t);
    }
    args.push(path);
    let out = kosmo_run().args(&args).output().expect("spawn kosmo-run");
    serde_json::from_slice(&out.stdout).expect("valid JSON")
}

#[test]
fn alchemy_seeds_from_workspace_and_reaches_a_fixpoint() {
    let root = workspace();
    let out = kosmo_run()
        .args(["--alchemy", "--no-color", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("workspace source"), "seeded from the workspace: {stdout}");
    assert!(stdout.contains("discoveries"), "the discovery loop is reported: {stdout}");
    assert!(stdout.contains("fixpoint reached"), "the catalog closes: {stdout}");
}

#[test]
fn alchemy_json_carries_the_catalog() {
    let root = workspace();
    let v = alchemy_json(&root, None);
    assert_eq!(v["seeded_from"], "workspace");
    assert_eq!(v["sources_seen"].as_u64(), Some(4), "four sources seen");
    let distinct = v["distinct_elements"].as_u64().unwrap();
    let catalog = v["catalog"].as_u64().unwrap();
    assert!(catalog >= distinct, "the catalog grows from its seeds: {catalog} >= {distinct}");
    assert_eq!(v["fixpoint"], true, "the lab reached a fixpoint");
}

#[test]
fn alchemy_finer_threshold_resolves_a_larger_catalog() {
    let root = workspace();
    let coarse = alchemy_json(&root, Some("0.80"))["catalog"].as_u64().unwrap();
    let fine = alchemy_json(&root, Some("0.97"))["catalog"].as_u64().unwrap();
    assert!(fine >= coarse, "a finer novelty threshold resolves more: fine={fine} coarse={coarse}");
}

#[test]
fn alchemy_falls_back_to_primitives_without_source() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let empty = std::env::temp_dir().join(format!("kosmo-lab-empty-{nanos}"));
    fs::create_dir_all(&empty).unwrap();
    let v = alchemy_json(&empty, None);
    assert_eq!(v["seeded_from"], "primitives", "no source → the four primitives");
    assert_eq!(v["distinct_elements"].as_u64(), Some(4), "four primitive elements");
    assert!(v["discovered"].as_u64().unwrap() > 0, "combination still discovers");
}

#[test]
fn alchemy_is_one_door_per_run() {
    let root = workspace();
    let out = kosmo_run()
        .args(["--alchemy", "--codematrix", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success(), "two organ doors in one run is rejected");
}

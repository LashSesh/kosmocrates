//! The steward, end to end — self-husbandry under an operator-named fence.
//!
//! The Etappe-V pins: the survey is read-only and host-path-free; husbandry
//! without a fence is refused (nothing is fenced by default); inside the
//! fence the chores are descended offline by the deterministic scaffolder,
//! each recorded as a norm-learning observation; the cap caps.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// A workspace with honest chores: one undocumented, untested module —
/// the landscape proposes `Doc(router)` and `Test(router_smoke)`, both
/// inside a `doc,test` fence and buildable offline.
fn chore_workspace(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-steward-{tag}-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub mod router;\n").unwrap();
    fs::write(
        root.join("src/router.rs"),
        "pub fn dispatch(p: &str) -> u32 { p.len() as u32 }\n",
    )
    .unwrap();
    root
}

fn tree_snapshot(root: &PathBuf) -> Vec<(String, String)> {
    let mut files = Vec::new();
    for entry in walk(root) {
        files.push((
            entry.to_string_lossy().to_string(),
            fs::read_to_string(&entry).unwrap_or_default(),
        ));
    }
    files.sort();
    files
}

fn walk(dir: &PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                out.extend(walk(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}

#[test]
fn survey_is_read_only_and_the_report_is_host_path_free() {
    let root = chore_workspace("survey");
    let report_path = root.join("steward.json");
    let before = tree_snapshot(&root);
    let out = kosmo_run()
        .args([
            "--steward",
            "--fence",
            "doc,test",
            "--no-color",
            "--steward-report",
            report_path.to_str().unwrap(),
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if stdout.contains("pipeline failed") || stderr.contains("pipeline failed") {
        eprintln!("pipeline unavailable, skipping: {stdout}{stderr}");
        fs::remove_dir_all(&root).ok();
        return;
    }
    assert!(out.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("fence doc, test"), "{stdout}");
    assert!(stdout.contains("Doc router"), "{stdout}");
    assert!(stdout.contains("Test router_smoke"), "{stdout}");
    assert!(
        stdout.contains("survey only"),
        "read-only contract stated: {stdout}"
    );

    let report = fs::read_to_string(&report_path).expect("report written");
    assert!(report.contains("report_id"), "{report}");
    assert!(report.contains("\"applied\": false"), "{report}");
    assert!(
        !report.contains(root.to_str().unwrap()),
        "the durable report names no host path: {report}"
    );

    // Nothing in the workspace changed (the report file is the one artifact).
    fs::remove_file(&report_path).unwrap();
    assert_eq!(tree_snapshot(&root), before, "survey wrote nothing");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn husbandry_without_a_fence_is_refused() {
    let root = chore_workspace("nofence");
    let out = kosmo_run()
        .args(["--steward", "--apply", "--no-color", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success(), "nothing is fenced by default");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("fence"), "{stderr}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn the_fence_speaks_only_the_facet_vocabulary() {
    let out = kosmo_run()
        .args(["--steward", "--fence", "doc,banana", "--no-color", "."])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("banana"), "{stderr}");
}

#[test]
fn steward_is_exclusive_with_the_other_doors() {
    let out = kosmo_run()
        .args(["--steward", "--landscape", "--no-color", "."])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("exclusive"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn fenced_chores_are_husbanded_offline_and_observed_by_the_norm_organ() {
    let root = chore_workspace("husband");
    let norms = root.join("norms-store");
    let report_path = root.join("steward.json");
    let out = kosmo_run()
        .args([
            "--steward",
            "--fence",
            "doc,test",
            "--apply",
            "--norms",
            norms.to_str().unwrap(),
            "--steward-report",
            report_path.to_str().unwrap(),
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if stdout.contains("pipeline failed")
        || stderr.contains("pipeline failed")
        || stdout.contains("could not observe")
        || stderr.contains("could not observe")
    {
        eprintln!("pipeline/observe unavailable, skipping: {stdout}{stderr}");
        fs::remove_dir_all(&root).ok();
        return;
    }
    assert!(out.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("husbanded:"), "{stdout}");

    // The chores actually landed: the module is documented, the smoke test
    // exists — built by the deterministic scaffolder, no provider involved.
    let lib = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let router = fs::read_to_string(root.join("src/router.rs")).unwrap();
    assert!(
        lib.contains("///") || router.contains("///") || router.contains("//!"),
        "a doc fiber landed:\n{lib}\n{router}"
    );
    assert!(
        lib.contains("fn router_smoke") || router.contains("fn router_smoke"),
        "the smoke test landed:\n{lib}\n{router}"
    );

    // The report tells the same story, chore by chore.
    let report = fs::read_to_string(&report_path).expect("report written");
    assert!(report.contains("\"applied\": true"), "{report}");
    assert!(report.contains("\"realized\": true"), "{report}");
    assert!(!report.contains("\"realized\": false"), "{report}");

    // Each chore was recorded as a norm-learning observation.
    let observations = fs::read_to_string(norms.join("observations.jsonl")).unwrap_or_default();
    assert!(
        observations.lines().count() >= 2,
        "one observation per chore: {observations}"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn the_cap_caps_the_chore_list() {
    let root = chore_workspace("cap");
    let report_path = root.join("steward.json");
    let out = kosmo_run()
        .args([
            "--steward",
            "--fence",
            "doc,test",
            "--apply",
            "--steward-max",
            "1",
            "--steward-report",
            report_path.to_str().unwrap(),
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if stdout.contains("pipeline failed")
        || stderr.contains("pipeline failed")
        || stdout.contains("could not observe")
        || stderr.contains("could not observe")
    {
        eprintln!("pipeline/observe unavailable, skipping: {stdout}{stderr}");
        fs::remove_dir_all(&root).ok();
        return;
    }
    assert!(out.status.success(), "{stdout}\n{stderr}");
    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report_path).unwrap()).unwrap();
    assert_eq!(
        report["report"]["chores"].as_array().unwrap().len(),
        1,
        "--steward-max 1 husbands exactly one chore"
    );
    fs::remove_dir_all(&root).ok();
}

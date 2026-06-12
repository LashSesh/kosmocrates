//! The venture, end to end — whole-system fabrication from dependent wishes.
//!
//! Pins (the Etappe-I exit criterion): a venture of five dependent stages
//! erects a runnable multi-component system in an empty crate, offline; the
//! preview is read-only; an aborted/finished session resumes by identity
//! (a changed spec is refused, never silently restarted); the venture door
//! is exclusive with the single-wish doors.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

fn scratch(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("kosmo-venture-{tag}-{nanos}"))
}

fn crate_fixture(root: &Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "").unwrap();
}

/// Five stages, functionally dependent: the doc stages can only realize
/// once the modules they document exist.
const SPEC: &str = r#"{
    "label": "tiny engine",
    "stages": [
        { "label": "engine", "wish": "a module engine and a function run_engine" },
        { "label": "store",  "wish": "a module store and a function put_record", "after": [0] },
        { "label": "docs",   "wish": "docs for engine and docs for store", "after": [0, 1] },
        { "label": "smoke",  "wish": "a test engine_smoke", "after": [0] },
        { "label": "wire",   "wish": "a capability engine-wired", "after": [1, 3] }
    ]
}"#;

#[test]
fn venture_erects_a_multi_component_system_offline() {
    let base = scratch("erect");
    let ws = base.join("ws");
    crate_fixture(&ws);
    let spec = base.join("venture.json");
    fs::write(&spec, SPEC).unwrap();
    let session = base.join("venture.session.json");

    let out = kosmo_run()
        .args([
            "--venture",
            spec.to_str().unwrap(),
            "--venture-session",
            session.to_str().unwrap(),
            "--apply",
            "--no-color",
            ws.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if stderr.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stderr}");
        fs::remove_dir_all(&base).ok();
        return;
    }
    assert_eq!(out.status.code(), Some(0), "{stdout}\n{stderr}");
    assert!(stdout.contains("venture: 5/5 realized"), "{stdout}");
    // The multi-component body actually stands.
    assert!(ws.join("src/engine.rs").exists(), "engine module erected");
    assert!(ws.join("src/store.rs").exists(), "store module erected");
    let lib = fs::read_to_string(ws.join("src/lib.rs")).unwrap();
    assert!(lib.contains("pub mod engine"), "{lib}");
    assert!(lib.contains("pub mod store"), "{lib}");
    assert!(lib.contains("fn engine_smoke"), "{lib}");
    // The session file is the durable staircase record.
    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&session).unwrap()).unwrap();
    let standings = saved["standings"].as_array().unwrap();
    assert_eq!(standings.len(), 5);
    assert!(standings.iter().all(|s| s == "Realized"), "{standings:?}");

    // Resume: a second run finds everything realized and re-descends nothing.
    let out = kosmo_run()
        .args([
            "--venture",
            spec.to_str().unwrap(),
            "--venture-session",
            session.to_str().unwrap(),
            "--apply",
            "--no-color",
            ws.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("venture: 5/5 realized"), "{stdout}");
    assert!(
        !stdout.contains("descending"),
        "a complete session resumes without re-descending: {stdout}"
    );

    // Identity guard: a changed spec may not adopt the old session.
    fs::write(&spec, SPEC.replace("tiny engine", "other venture")).unwrap();
    let out = kosmo_run()
        .args([
            "--venture",
            spec.to_str().unwrap(),
            "--venture-session",
            session.to_str().unwrap(),
            "--apply",
            "--no-color",
            ws.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("different venture"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    fs::remove_dir_all(&base).ok();
}

#[test]
fn preview_is_read_only_and_shows_the_staircase() {
    let base = scratch("preview");
    let ws = base.join("ws");
    crate_fixture(&ws);
    let spec = base.join("venture.json");
    fs::write(&spec, SPEC).unwrap();
    let before = fs::read_to_string(ws.join("src/lib.rs")).unwrap();

    let out = kosmo_run()
        .args([
            "--venture",
            spec.to_str().unwrap(),
            "--no-color",
            ws.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("order: engine → store → docs → smoke → wire"),
        "the deterministic staircase renders: {stdout}"
    );
    assert!(stdout.contains("read-only"), "{stdout}");
    assert!(stdout.contains("measured: 0/"), "stages measured: {stdout}");
    assert_eq!(
        fs::read_to_string(ws.join("src/lib.rs")).unwrap(),
        before,
        "nothing written without --apply"
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn venture_door_guards_its_flags_and_its_spec() {
    let base = scratch("guards");
    let ws = base.join("ws");
    crate_fixture(&ws);
    let spec = base.join("venture.json");
    fs::write(&spec, SPEC).unwrap();

    // Exclusive with the single-wish doors.
    let out = kosmo_run()
        .args([
            "--venture",
            spec.to_str().unwrap(),
            "--wish",
            "a crate x",
            ws.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("exclusive"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // A cyclic spec is refused with the stage labels named.
    let cyclic = r#"{"label":"x","stages":[
        {"label":"a","wish":"a module a1","after":[1]},
        {"label":"b","wish":"a module b1","after":[0]}
    ]}"#;
    fs::write(&spec, cyclic).unwrap();
    let out = kosmo_run()
        .args(["--venture", spec.to_str().unwrap(), ws.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("cycle"), "{stderr}");

    fs::remove_dir_all(&base).ok();
}

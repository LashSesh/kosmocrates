//! The doors door, end to end — the binary's self-description.
//!
//! The Spreizung-I pins: `--doors` answers without a workspace, in both
//! shapes (operator text, machine JSON), deterministically — the catalog
//! is the same truth on every invocation.

use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

#[test]
fn doors_answers_offline_in_both_shapes() {
    // The text shape: operator-readable, governance visible.
    let out = kosmo_run()
        .args(["--doors", "--no-color"])
        .output()
        .expect("spawn kosmo-run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("docking surface"), "{text}");
    assert!(text.contains("--steward"), "{text}");
    assert!(text.contains("[governance-act]"), "{text}");
    assert!(text.contains("[read-only]"), "{text}");

    // The machine shape: a content-addressed catalog that parses.
    let out = kosmo_run()
        .args(["--doors", "--json"])
        .output()
        .expect("spawn kosmo-run");
    assert!(out.status.success());
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("catalog json");
    assert!(!doc["catalog_id"].as_str().unwrap().is_empty());
    let doors = doc["doors"].as_array().unwrap();
    assert!(doors.len() >= 13, "all modes are doors");
    assert!(
        doors.iter().any(|d| d["name"] == "--wish"),
        "the wish door is described"
    );
    assert!(
        doors.iter().any(|d| d["name"] == "--doors"),
        "the catalog describes itself"
    );
}

#[test]
fn the_catalog_is_the_same_truth_every_time() {
    let run = || {
        let out = kosmo_run()
            .args(["--doors", "--json"])
            .output()
            .expect("spawn kosmo-run");
        assert!(out.status.success());
        out.stdout
    };
    assert_eq!(run(), run(), "self-description is deterministic");
}

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
fn federation_merges_verified_catalogs_and_refuses_tampered_ones() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("kosmo-doors-federate-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();

    // The artifact this binary itself would emit…
    let own = kosmo_run()
        .args(["--doors", "--json"])
        .output()
        .expect("spawn kosmo-run");
    let own_doc: serde_json::Value = serde_json::from_slice(&own.stdout).unwrap();
    let own_len = own_doc["doors"].as_array().unwrap().len();
    let a = dir.join("own.json");
    std::fs::write(&a, &own.stdout).unwrap();

    // …and one a foreign surface would emit, built from the shared vocabulary.
    let foreign = kosmo_core::DoorCatalog::new(vec![kosmo_core::Door::new(
        kosmo_core::DoorSurface::Http {
            binary: "demo-server".into(),
            method: "GET".into(),
        },
        "/api/x",
        vec![],
        "a foreign surface's door",
        vec![],
        kosmo_core::DoorGovernance::ReadOnly,
        vec![],
    )]);
    let b = dir.join("foreign.json");
    std::fs::write(&b, serde_json::to_string(&foreign).unwrap()).unwrap();

    // Merging own-with-own dedups; the foreign door joins the inventory.
    let merge_arg = format!("{},{}", a.display(), b.display());
    let out = kosmo_run()
        .args(["--doors-merge", &merge_arg, "--json"])
        .output()
        .expect("spawn kosmo-run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let merged: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        merged["doors"].as_array().unwrap().len(),
        own_len + 1,
        "own doors collapse, the foreign one joins"
    );

    // The rendered inventory names both surfaces.
    let out = kosmo_run()
        .args(["--doors-merge", &merge_arg, "--no-color"])
        .output()
        .expect("spawn kosmo-run");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("federated"), "{text}");
    assert!(text.contains("demo-server"), "{text}");
    assert!(text.contains("GET /api/x"), "{text}");

    // A tampered catalog no longer recomputes — refused by name.
    let tampered = std::fs::read_to_string(&b)
        .unwrap()
        .replace("a foreign surface's door", "a REWORDED promise");
    std::fs::write(&b, tampered).unwrap();
    let out = kosmo_run()
        .args(["--doors-merge", &merge_arg, "--no-color"])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("verification"), "{stderr}");
    assert!(stderr.contains("foreign.json"), "names the file: {stderr}");

    std::fs::remove_dir_all(&dir).ok();
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

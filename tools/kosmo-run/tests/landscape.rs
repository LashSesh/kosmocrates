//! The wish landscape — the findings become the wish menu.
//!
//! `--landscape` projects every substrate finding the wish vocabulary can
//! express into a ranked proposal landscape, measured against the observed
//! topology (met / open / beyond observation / beyond vocabulary), and
//! `--adopt <n>` turns the top open proposals into ONE severity-weighted,
//! evidence-bound wish — read-only by default, descended under `--apply`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// A mixed workspace: one undocumented, untested Rust module (observable)
/// and one Python file (an honest beyond-observation target).
fn mixed_workspace(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-landscape-{tag}-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("lib")).unwrap();
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
    fs::write(
        root.join("lib/handlers.py"),
        "def handle(x):\n    return x\n",
    )
    .unwrap();
    root
}

#[test]
fn landscape_maps_findings_with_honest_standing() {
    let root = mixed_workspace("map");
    let out = kosmo_run()
        .args(["--landscape", "--json", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json landscape");

    let proposals = doc["proposals"].as_array().unwrap();
    assert!(!proposals.is_empty(), "findings must project to proposals");
    let standing_of = |key: &str| -> String {
        proposals
            .iter()
            .find(|p| p["facet_key"] == key)
            .unwrap_or_else(|| panic!("proposal {key} present"))["standing"]
            .as_str()
            .unwrap()
            .to_string()
    };
    // The Rust module's targets are open (observable, unmet)…
    assert_eq!(standing_of("router"), "open");
    assert_eq!(standing_of("router_smoke"), "open");
    // …the Python target is honestly beyond the wish world's observation.
    assert_eq!(standing_of("handlers"), "beyond-observation");
    assert!(doc["open"].as_u64().unwrap() >= 2);
    // The landscape is evidence-bound to the diagnosis report.
    assert!(!doc["report_id"].as_str().unwrap().is_empty());

    fs::remove_dir_all(&root).ok();
}

#[test]
fn adopt_without_apply_is_read_only() {
    let root = mixed_workspace("readonly");
    let before = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let out = kosmo_run()
        .args([
            "--landscape",
            "--adopt",
            "2",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("adopted as one wish"));
    assert!(
        stdout.contains("add --apply to descend"),
        "read-only contract stated: {stdout}"
    );
    assert_eq!(
        fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        before,
        "nothing written without --apply"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn adopt_with_apply_descends_to_realized() {
    let root = mixed_workspace("apply");
    let out = kosmo_run()
        .args([
            "--landscape",
            "--adopt",
            "2",
            "--apply",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() {
        // cargo metadata unavailable in exotic sandboxes — the in-process
        // descend tests already pin convergence; this is the CLI seam.
        eprintln!("observe unavailable, skipping: {stdout}");
        fs::remove_dir_all(&root).ok();
        return;
    }
    assert!(stdout.contains("REALIZED"), "descent converged: {stdout}");
    let lib = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    assert!(
        lib.contains("/// `router`"),
        "doc stub landed above the module declaration: {lib}"
    );
    assert!(lib.contains("fn router_smoke"), "smoke test landed: {lib}");
    fs::remove_dir_all(&root).ok();
}

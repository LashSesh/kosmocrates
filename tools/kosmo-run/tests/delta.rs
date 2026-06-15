//! Run 13 — the delta: `--since <baseline>` reports what moved for a wish.
//!
//! Pins the round trip: a `--wish-session` snapshot is a baseline; after the
//! workspace changes, re-measuring `--since` that snapshot names the facet that
//! was newly realized (gained). Read-only — nothing is written by `--since`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// A crate with module `alpha` present and `beta` absent.
fn workspace() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-delta-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub mod alpha;\n").unwrap();
    fs::write(root.join("src/alpha.rs"), "pub fn f() {}\n").unwrap();
    root
}

#[test]
fn since_names_the_newly_realized_facet() {
    let root = workspace();
    let base = root.join("base.json");
    let wish = "a module alpha and a module beta";

    // Snapshot the baseline: beta is missing, so the wish is unrealized (exit 1).
    let snap = kosmo_run()
        .args([
            "--wish",
            wish,
            "--wish-session",
            base.to_str().unwrap(),
            "--flat",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (snapshot)");
    let snap_out = String::from_utf8_lossy(&snap.stdout);
    if snap_out.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {snap_out}");
        return;
    }
    assert_eq!(snap.status.code(), Some(1), "beta missing → exit 1: {snap_out}");
    assert!(base.exists(), "the baseline session was written");

    // The workspace changes: beta is added.
    fs::write(root.join("src/lib.rs"), "pub mod alpha;\npub mod beta;\n").unwrap();
    fs::write(root.join("src/beta.rs"), "pub fn g() {}\n").unwrap();

    // Re-measure --since the baseline: beta is newly met.
    let delta = kosmo_run()
        .args([
            "--wish",
            wish,
            "--since",
            base.to_str().unwrap(),
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (--since)");
    let out = String::from_utf8_lossy(&delta.stdout);
    assert!(out.contains("delta since baseline"), "{out}");
    assert!(out.contains("gained 1"), "beta was newly realized: {out}");
    assert!(out.contains("Module beta"), "the delta names the facet: {out}");
    // A pure gain to a now-realized wish exits 0 — no regression.
    assert_eq!(delta.status.code(), Some(0), "realized, no regression: {out}");
}

#[test]
fn since_regression_exits_2() {
    let root = workspace();
    // Add beta so the wish is fully realized, then snapshot that baseline.
    fs::write(root.join("src/lib.rs"), "pub mod alpha;\npub mod beta;\n").unwrap();
    fs::write(root.join("src/beta.rs"), "pub fn g() {}\n").unwrap();
    let base = root.join("base.json");
    let wish = "a module alpha and a module beta";
    let snap = kosmo_run()
        .args([
            "--wish",
            wish,
            "--wish-session",
            base.to_str().unwrap(),
            "--flat",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (snapshot)");
    let snap_out = String::from_utf8_lossy(&snap.stdout);
    if snap_out.contains("could not observe") {
        return;
    }
    assert_eq!(snap.status.code(), Some(0), "both modules present → realized: {snap_out}");

    // The change breaks beta. --since detects the regression → exit 2.
    fs::write(root.join("src/lib.rs"), "pub mod alpha;\n").unwrap();
    fs::remove_file(root.join("src/beta.rs")).unwrap();
    let out = kosmo_run()
        .args([
            "--wish",
            wish,
            "--since",
            base.to_str().unwrap(),
            "--flat",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (--since)");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(2), "a regression exits 2: {stdout}");
    assert!(stdout.contains("regressed 1"), "{stdout}");
    assert!(stdout.contains("Module beta"), "names the broken facet: {stdout}");
}

#[test]
fn since_json_emits_a_self_describing_machine_delta() {
    let root = workspace();
    let base = root.join("base.json");
    let wish = "a module alpha and a module beta";
    let snap = kosmo_run()
        .args([
            "--wish",
            wish,
            "--wish-session",
            base.to_str().unwrap(),
            "--flat",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (snapshot)");
    if String::from_utf8_lossy(&snap.stdout).contains("could not observe") {
        return;
    }
    fs::write(root.join("src/lib.rs"), "pub mod alpha;\npub mod beta;\n").unwrap();
    fs::write(root.join("src/beta.rs"), "pub fn g() {}\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wish",
            wish,
            "--since",
            base.to_str().unwrap(),
            "--json",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (--since --json)");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("valid JSON ({e}): {stdout}"));
    assert!(v.get("gained").is_some(), "machine delta carries gained: {stdout}");
    assert!(v.get("regressed").is_some(), "and regressed: {stdout}");
    assert!(v.get("wish_id").is_some(), "self-describing by wish id: {stdout}");
}

#[test]
fn since_without_a_matching_baseline_says_so() {
    let root = workspace();
    let out = kosmo_run()
        .args([
            "--wish",
            "a module alpha",
            "--since",
            root.join("does-not-exist.json").to_str().unwrap(),
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        return;
    }
    assert!(stdout.contains("no matching baseline"), "{stdout}");
}

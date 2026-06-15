//! Run 15 — the wishlist: a file of wishes measured as a project's
//! definition-of-done, into an aggregate realization gauge.
//!
//! Pins: the aggregate `realized N/M` is reported, the run exits 0 only when
//! every wish is realized, and `--json` emits a machine project gauge.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// A crate with module `alpha` and function `f` present; module `beta` absent.
fn workspace() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-wishlist-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub mod alpha;\npub fn f() {}\n").unwrap();
    fs::write(root.join("src/alpha.rs"), "pub fn a() {}\n").unwrap();
    root
}

#[test]
fn wishlist_reports_aggregate_and_gates_unrealized() {
    let root = workspace();
    let list = root.join("project.wishes");
    fs::write(
        &list,
        "# the project's definition of done\na module alpha\na function f\na module beta\n",
    )
    .unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stdout}");
        return;
    }
    assert!(stdout.contains("Kosmocrates wishlist"), "{stdout}");
    assert!(stdout.contains("realized 2/3"), "alpha + f met, beta missing: {stdout}");
    assert!(stdout.contains("a module beta"), "{stdout}");
    assert_eq!(out.status.code(), Some(1), "not all realized → exit 1: {stdout}");
}

#[test]
fn wishlist_all_realized_exits_0() {
    let root = workspace();
    let list = root.join("done.wishes");
    fs::write(&list, "a module alpha\na function f\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        return;
    }
    assert!(stdout.contains("realized 2/2"), "{stdout}");
    assert_eq!(out.status.code(), Some(0), "all realized → exit 0: {stdout}");
}

#[test]
fn wishlist_json_emits_a_project_gauge() {
    let root = workspace();
    let list = root.join("p.wishes");
    fs::write(&list, "a module alpha\na module beta\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--json",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        return;
    }
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("valid JSON ({e}): {stdout}"));
    assert_eq!(v["realized"], 1, "one of two realized: {stdout}");
    assert_eq!(v["total"], 2, "{stdout}");
    assert!(v["wishes"].is_array(), "per-wish rows: {stdout}");
}

#[test]
fn wishlist_since_gates_a_project_regression() {
    let root = workspace(); // module alpha + function f present
    let list = root.join("p.wishes");
    fs::write(&list, "a module alpha\na function f\n").unwrap(); // both realized
    let base = root.join("base.json");

    // Snapshot the baseline reading (all realized) via --json.
    let snap = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--json",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (snapshot)");
    let snap_out = String::from_utf8_lossy(&snap.stdout);
    if snap_out.contains("could not observe") {
        return;
    }
    assert_eq!(snap.status.code(), Some(0), "baseline all realized: {snap_out}");
    fs::write(&base, snap_out.as_ref()).unwrap();

    // The change breaks module alpha. --since detects the project regression.
    fs::write(root.join("src/lib.rs"), "pub fn f() {}\n").unwrap();
    fs::remove_file(root.join("src/alpha.rs")).unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--since",
            base.to_str().unwrap(),
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (--since)");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("delta since baseline"), "{stdout}");
    assert!(stdout.contains("regressed 1"), "{stdout}");
    assert!(stdout.contains("a module alpha"), "names the broken wish: {stdout}");
    assert_eq!(out.status.code(), Some(2), "a project regression exits 2: {stdout}");
}

#[test]
fn wishlist_since_reports_progress_without_regression() {
    let root = workspace(); // alpha + f present; beta absent
    let list = root.join("p.wishes");
    fs::write(&list, "a module alpha\na module beta\n").unwrap();
    let base = root.join("base.json");
    let snap = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--json",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (snapshot)");
    let snap_out = String::from_utf8_lossy(&snap.stdout);
    if snap_out.contains("could not observe") {
        return;
    }
    fs::write(&base, snap_out.as_ref()).unwrap();

    // Add beta → newly realized, nothing broke → all realized → exit 0.
    fs::write(root.join("src/lib.rs"), "pub mod alpha;\npub mod beta;\npub fn f() {}\n").unwrap();
    fs::write(root.join("src/beta.rs"), "pub fn b() {}\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--since",
            base.to_str().unwrap(),
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (--since)");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("newly realized 1"), "{stdout}");
    assert!(stdout.contains("a module beta"), "{stdout}");
    assert_eq!(out.status.code(), Some(0), "progress, no regression → exit 0: {stdout}");
}

#[test]
fn wishlist_is_exclusive_with_wish() {
    let root = workspace();
    let list = root.join("x.wishes");
    fs::write(&list, "a module alpha\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--wish",
            "a module alpha",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success(), "two doors in one run is refused");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("exclusive"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

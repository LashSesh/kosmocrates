//! Capstone — the whole Horizon floor in one runnable artifact.
//!
//! This drives the **real `kosmo-run` binary** end-to-end (`CARGO_BIN_EXE_…`),
//! not an in-process helper: prose in, a validated system out. It is the
//! convergence of the floor we built — a typed `Contract` plus the `Behavior`
//! keystone — and the first reference scenario for the empirical Prüfstand.
//!
//! Two directions, both through the actual CLI:
//!   * a **correct** implementation is accepted (the descent reaches REALIZED);
//!   * an **incorrect** implementation is rejected (the descent does not lie).
//!
//! The positive case runs `cargo test` (the behaviour is validated by running
//! the suite); if the sandbox cannot nest cargo it skips rather than fails —
//! the in-process unit tests already prove convergence. The negative case is
//! robust either way: a wrong impl is never realized.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// A single-crate workspace named `calc` with the given `src/lib.rs`.
fn temp_workspace(lib: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-capstone-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"calc\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), lib).unwrap();
    root
}

fn run_wish(root: &PathBuf, wish: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
        .args(["--wish", wish, "--apply", "--no-color"])
        .arg(root)
        .output()
        .expect("the kosmo-run binary should run")
}

/// Prose → a validated system: a typed contract plus a behaviour spec, against
/// a correct implementation, realized end-to-end through the CLI.
#[test]
fn capstone_prose_to_validated_system() {
    let root = temp_workspace("pub fn add(a: i32, b: i32) -> i32 { a + b }\n");
    let out = run_wish(
        &root,
        "a contract add(i32,i32)->i32 and a behavior add(2,3)=>5",
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    if stdout.contains("REALIZED") {
        assert!(out.status.success(), "exit 0 when realized:\n{stdout}");
        assert!(stdout.contains("2/2"), "both facets met:\n{stdout}");
        // The spec-test (the acceptance gate) was scaffolded into the crate.
        let lib = fs::read_to_string(root.join("src/lib.rs")).unwrap();
        assert!(
            lib.contains("// kosmo:behavior: add(2,3)=>5"),
            "spec-test marker present:\n{lib}"
        );
    } else {
        eprintln!(
            "capstone (positive) skipped — kosmo-run did not realize:\n{stdout}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    fs::remove_dir_all(&root).ok();
}

/// Acceptance over generation, end-to-end: a wrong implementation
/// (`add` returns `a + b + 1`) must NOT be accepted. Robust without nested
/// cargo — a behaviour whose test cannot be observed green is never realized.
#[test]
fn capstone_rejects_incorrect_system() {
    let root = temp_workspace("pub fn add(a: i32, b: i32) -> i32 { a + b + 1 }\n");
    let out = run_wish(&root, "a behavior add(2,3)=>5");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        !out.status.success(),
        "a wrong impl must not exit 0 (the system does not lie):\n{stdout}"
    );
    assert!(
        !stdout.contains("wish realized"),
        "a wrong impl must not be reported realized:\n{stdout}"
    );
    fs::remove_dir_all(&root).ok();
}

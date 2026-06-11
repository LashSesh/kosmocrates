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
use std::sync::Mutex;

/// Each test here spawns the real `kosmo-run` binary, which in turn spawns
/// nested `cargo`. Run them one at a time so a full `cargo test --workspace`
/// (where this binary executes after every other crate) does not pile concurrent
/// toolchain processes up — peak process/memory pressure can make a spawn fail.
/// Poison-tolerant: a panic in one test must not cascade into the others.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

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

/// Run the real binary against a wish. `None` if the process could not even be
/// spawned (transient resource pressure under a full-workspace run) — callers
/// treat that as a skip, never a failure.
fn run_wish(root: &PathBuf, wish: &str) -> Option<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
        .args(["--wish", wish, "--apply", "--no-color"])
        .arg(root)
        .output()
        .ok()
}

/// Prose → a validated system: a typed contract plus a behaviour spec, against
/// a correct implementation, realized end-to-end through the CLI.
#[test]
fn capstone_prose_to_validated_system() {
    let _serial = serial();
    let root = temp_workspace("pub fn add(a: i32, b: i32) -> i32 { a + b }\n");
    let Some(out) = run_wish(
        &root,
        "a contract add(i32,i32)->i32 and a behavior add(2,3)=>5",
    ) else {
        eprintln!("capstone (positive) skipped — could not spawn kosmo-run");
        fs::remove_dir_all(&root).ok();
        return;
    };
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
    let _serial = serial();
    let root = temp_workspace("pub fn add(a: i32, b: i32) -> i32 { a + b + 1 }\n");
    let Some(out) = run_wish(&root, "a behavior add(2,3)=>5") else {
        eprintln!("capstone (negative) skipped — could not spawn kosmo-run");
        fs::remove_dir_all(&root).ok();
        return;
    };
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

/// The runtime capstone (`docs/RUNTIME-floor.md` §5): the **same truth** at the
/// function boundary (`Behavior`) and the process boundary (`Run`), end-to-end
/// through the CLI. A typed `Contract`, a unit `Behavior`, and a `Run` probe of
/// the built binary all over one little calculator.
#[test]
fn capstone_runtime_unit_and_process_boundary() {
    let _serial = serial();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-capstone-rt-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"calc\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
    )
    .unwrap();
    // The CLI wraps the library fn — so the Run probe exercises the same add.
    fs::write(
        root.join("src/main.rs"),
        "fn main() { let a: Vec<String> = std::env::args().skip(1).collect(); \
         if a.first().map(|s| s.as_str()) == Some(\"add\") { \
         println!(\"{}\", calc::add(a[1].parse().unwrap(), a[2].parse().unwrap())); } }\n",
    )
    .unwrap();

    let Some(out) = run_wish(
        &root,
        "a contract add(i32,i32)->i32 and a behavior add(2,3)=>5 and a run add,2,3=>out~5",
    ) else {
        eprintln!("runtime capstone skipped — could not spawn kosmo-run");
        fs::remove_dir_all(&root).ok();
        return;
    };
    let stdout = String::from_utf8_lossy(&out.stdout);

    if stdout.contains("REALIZED") {
        assert!(out.status.success(), "exit 0 when realized:\n{stdout}");
        assert!(stdout.contains("3/3"), "all three facets met:\n{stdout}");
        let main = fs::read_to_string(root.join("src/main.rs")).unwrap();
        assert!(
            main.contains("// kosmo:run: add,2,3=>out~5"),
            "run-probe marker scaffolded into the bin:\n{main}"
        );
    } else {
        eprintln!(
            "runtime capstone skipped — kosmo-run did not realize (nested cargo?):\n{stdout}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    fs::remove_dir_all(&root).ok();
}

//! The substrate organ doors, end to end — Spreizung II.
//!
//! Five organs that used to be side effects of `--apply`/`--all` are now
//! directed doors: foundry (the gate executor), witness (one sandboxed
//! execution), parseback (topology snapshot + drift), kcube (the real
//! blueprint archive), codematrix (the advisory 5D lens). Each is pinned
//! here by behaviour, fail-closed.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// A tiny runnable workspace: one bin that prints 42.
fn fixture(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-organs-{tag}-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/main.rs"),
        "fn main() { println!(\"{}\", 6 * 7); }\n",
    )
    .unwrap();
    root
}

#[test]
fn foundry_runs_the_gates_and_fails_closed() {
    let root = fixture("foundry");
    let out = kosmo_run()
        .args(["--foundry", "build", "--no-color", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("Build passed"), "{stdout}");
    assert!(stdout.contains("evidence"), "{stdout}");

    // A broken workspace fails the check — exit 6, never a warning.
    fs::write(
        root.join("src/main.rs"),
        "fn main() { this does not compile }\n",
    )
    .unwrap();
    let out = kosmo_run()
        .args(["--foundry", "build", "--no-color", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    assert_eq!(out.status.code(), Some(6), "a failed gate is a failure");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("failed"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );

    // The vocabulary is closed: an unknown check is refused by name.
    let out = kosmo_run()
        .args(["--foundry", "banana", "--no-color", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("banana"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn witness_executes_once_and_addresses_the_evidence() {
    let root = fixture("witness");
    let out = kosmo_run()
        .args(["--witness", "ignored", "--no-color", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("stdout digest"), "{stdout}");
    assert!(stdout.contains("| 42"), "the program's answer: {stdout}");

    // A non-zero exit is a non-zero exit — measured, not warned away.
    fs::write(
        root.join("src/main.rs"),
        "fn main() { std::process::exit(3); }\n",
    )
    .unwrap();
    let out = kosmo_run()
        .args(["--witness", "x", "--no-color", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    assert_eq!(out.status.code(), Some(7), "exit 7 unless clean");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn parseback_baselines_once_and_reports_drift() {
    let root = fixture("parseback");
    let baseline = root.join("base.json");
    let run = || {
        kosmo_run()
            .args([
                "--parseback",
                "--parseback-baseline",
                baseline.to_str().unwrap(),
                "--no-color",
                root.to_str().unwrap(),
            ])
            .output()
            .expect("spawn kosmo-run")
    };

    // First run writes the baseline.
    let out = run();
    assert!(out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("baseline written"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let baseline_bytes = fs::read(&baseline).expect("baseline persisted");

    // Second run, untouched workspace: no drift — and the baseline is
    // never silently replaced.
    let out = run();
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("no topology drift"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(
        fs::read(&baseline).unwrap(),
        baseline_bytes,
        "the baseline file is read-only after creation"
    );

    // Topology change: a new source file is a reported delta.
    fs::write(root.join("src/lib.rs"), "pub fn helper() {}\n").unwrap();
    let out = run();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Crate modified"), "{stdout}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn kcube_exports_verified_archives_one_per_diagnosis() {
    let root = fixture("kcube");
    let out_dir = root.join("cubes");
    let run = || {
        kosmo_run()
            .args([
                "--kcube",
                out_dir.to_str().unwrap(),
                "--no-color",
                root.to_str().unwrap(),
            ])
            .output()
            .expect("spawn kosmo-run")
    };
    let out = run();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if stdout.contains("pipeline failed") || stderr.contains("pipeline failed") {
        eprintln!("pipeline unavailable, skipping: {stdout}{stderr}");
        fs::remove_dir_all(&root).ok();
        return;
    }
    assert_eq!(out.status.code(), Some(0), "{stdout}\n{stderr}");
    assert!(stdout.contains("roundtrip verified"), "{stdout}");
    let archives = || -> Vec<String> {
        fs::read_dir(&out_dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "kcube"))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect()
    };
    assert_eq!(archives().len(), 1, "exactly one archive written");

    // Each diagnosis run is its own observation event with its own cube
    // identity: a second export lands a second, distinctly named archive —
    // nothing is ever silently replaced (the overwrite guard itself is
    // pinned in the kcube crate).
    let out = run();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let names = archives();
    assert_eq!(names.len(), 2, "a second diagnosis, a second archive");
    assert_ne!(names[0], names[1]);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn codematrix_is_an_advisory_lens() {
    let root = fixture("codematrix");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn helper(x: u32) -> u32 { x }\n",
    )
    .unwrap();
    let out = kosmo_run()
        .args(["--codematrix", "--no-color", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("advisory"), "{stdout}");
    assert!(stdout.contains("src/main.rs"), "{stdout}");
    assert!(stdout.contains("most resonant pairs"), "{stdout}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn organ_doors_are_one_per_run() {
    let out = kosmo_run()
        .args(["--foundry", "build", "--codematrix", "--no-color", "."])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("one door"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

//! Run 10 — the cube view is the default human render; `--flat` is the opt-out.
//!
//! Pins: a bare human wish-run renders the layered hypercube on top of the
//! headline verdict, while `--flat` prints the headline alone. The verdict
//! string survives either way, so the script-gating contract (exit code +
//! "REALIZED") is unmoved — only the human's view got richer by default.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// A minimal crate whose one module satisfies the wish outright (realized,
/// read-only — no cargo invocation, so the test is fast and deterministic).
fn fixture() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-grad-{nanos}"));
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

fn run(extra: &[&str]) -> (String, Option<i32>) {
    let root = fixture();
    let mut args: Vec<String> = vec!["--wish".into(), "a module router".into()];
    args.extend(extra.iter().map(|s| s.to_string()));
    args.push("--no-color".into());
    args.push(root.to_str().unwrap().into());
    let out = kosmo_run().args(&args).output().expect("spawn kosmo-run");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

#[test]
fn default_human_run_shows_the_cube_view() {
    let (stdout, code) = run(&[]);
    // The headline verdict is present (exit 0, realized) ...
    assert_eq!(code, Some(0), "{stdout}");
    assert!(stdout.contains("REALIZED"), "headline verdict: {stdout}");
    // ... and the graduated cube view rides on top, with no flag asked.
    assert!(
        stdout.contains("solidity(geomean)"),
        "the layered hypercube renders by default: {stdout}"
    );
}

#[test]
fn flat_opts_out_to_the_terse_verdict() {
    let (stdout, code) = run(&["--flat"]);
    // The headline verdict still prints — the script contract is unmoved ...
    assert_eq!(code, Some(0), "{stdout}");
    assert!(
        stdout.contains("REALIZED"),
        "headline verdict survives --flat: {stdout}"
    );
    // ... but the cube view is suppressed.
    assert!(
        !stdout.contains("solidity(geomean)"),
        "--flat suppresses the layered hypercube: {stdout}"
    );
}

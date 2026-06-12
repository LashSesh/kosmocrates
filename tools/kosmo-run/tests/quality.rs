//! The quality axes, end to end — speed as a measurable, fail-closed facet.
//!
//! The Etappe-II pin: a wish carrying a runtime budget (`ms<N`) is observed
//! by *executing the program* and timing it under the sandbox witness — a
//! generous budget is realized, a blown budget is a miss, never a warning.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// A runnable fixture whose main sleeps ~60ms and prints a marker — slow
/// enough that a 10ms budget deterministically blows, fast enough that a
/// 60s budget deterministically holds.
fn slow_bin_fixture(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-quality-{tag}-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/main.rs"),
        "// kosmo:run: hi=>out~hi,ms<60000\n// kosmo:run: hi=>out~hi,ms<10\nfn main() {\n    std::thread::sleep(std::time::Duration::from_millis(60));\n    println!(\"hi\");\n}\n",
    )
    .unwrap();
    root
}

#[test]
fn runtime_budget_is_measured_failclosed() {
    let root = slow_bin_fixture("budget");

    // Within budget: the program runs, prints, and stays under 60s.
    let out = kosmo_run()
        .args([
            "--wish",
            "a run hi=>out~hi,ms<60000",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if stderr.contains("could not observe") || stdout.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stdout}{stderr}");
        fs::remove_dir_all(&root).ok();
        return;
    }
    assert_eq!(out.status.code(), Some(0), "{stdout}\n{stderr}");
    assert!(stdout.contains("REALIZED"), "{stdout}");

    // Blown budget: the same program, the same output — but 60ms > 10ms.
    // Speed is now a facet: the miss is measured, never warned away.
    let out = kosmo_run()
        .args([
            "--wish",
            "a run hi=>out~hi,ms<10",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("missing"), "{stdout}");
    assert!(stdout.contains("ms<10"), "{stdout}");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn reforge_refuses_to_run_without_a_real_provider() {
    // The bench's honesty boundary, pinned offline: re-forging implements
    // behaviour, so without the one sanctioned non-deterministic worker it
    // refuses — it never pretends.
    let out = kosmo_run()
        .arg("--reforge")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("CEREBRAS_API_KEY")
        .env_remove("KOSMO_LLM_API_KEY")
        .env_remove("KOSMO_LLM_PROVIDER")
        .env_remove("KOSMO_LLM_BASE_URL")
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("provider"), "{stderr}");

    let out = kosmo_run()
        .args(["--reforge", "--provider", "mock"])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("forging theater"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

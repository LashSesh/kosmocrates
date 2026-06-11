//! Memory-grounded synthesis — the CLI contract.
//!
//! `--ledger <path>` attaches the anchored Infinity-Ledger memory to the run:
//! every synthesis request is grounded in recalled crystals and every patch
//! cites what it received. The contract under test here is the fail-closed
//! half: memory explicitly asked for must exist — a missing ledger is a hard
//! error, never a silent memory-free run. (The positive recall path is
//! covered in-process by `pse-adapter-kosmo` and `kosmo-agent`; the prompt
//! rendering by `kosmo-synthesizer-llm`.)

use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

#[test]
fn missing_ledger_is_a_hard_error() {
    let out = kosmo_run()
        .args([
            "--provider",
            "mock",
            "--ledger",
            "/nonexistent/kosmo-memory-xyz",
            ".",
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(
        !out.status.success(),
        "a missing ledger must fail the run, not degrade it"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("nothing to recall"),
        "stderr must name the contract, got: {stderr}"
    );
}

#[test]
fn help_documents_the_memory_flags() {
    let out = kosmo_run().arg("--help").output().expect("spawn kosmo-run");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--ledger <path>"));
    assert!(stdout.contains("--ground-top <n>"));
    assert!(stdout.contains("MEMORY"));
}

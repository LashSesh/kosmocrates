//! Swarm CLI contract: `--swarm <n>` arms the consensus ensemble and
//! refuses consensus theater.

use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

#[test]
fn swarm_refuses_the_mock_provider() {
    let out = kosmo_run()
        .args(["--provider", "mock", "--swarm", "3", "."])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success(), "consensus theater must be refused");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("real provider"),
        "names the contract: {stderr}"
    );
}

#[test]
fn help_documents_the_swarm_flag() {
    let out = kosmo_run().arg("--help").output().expect("spawn");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--swarm <n>"));
    assert!(stdout.contains("divergent ensemble"));
}
